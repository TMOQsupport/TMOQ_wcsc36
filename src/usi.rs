// usi.rs - USIエンジン（やねうら王classic-lite + 簡易Lazy SMP）
//
// 探索アルゴリズム: やねうら王mini/classic を参考
//   1. Razoring
//   2. Null Move Pruning（完全実装）
//   3. Move Count Based Futility Pruning
//   4. Lazy SMP（簡易版: 複数スレッドで独立探索、置換表共有）
//   5. LMR (Late Move Reduction)
//   6. Killer Move + Counter Move + History Heuristic
//   7. mate1ply() - 1手詰め判定（オプションA）
//   8. Check Extension - 王手で1手延長（オプションA）
//
// 評価関数: 駒得のみ（Material-only）
//   - やねうら王その1の駒価値（15パラメータ）
//   - KPP/KKP型評価関数は未実装
//
// Lazy SMP の実装方針:
//   - 置換表を Arc<Mutex<TT>> で共有
//   - 全スレッドが depth 1 から反復深化
//   - 最終的に completedDepth が最も深いスレッドの指し手を採用

use crate::position::Position;
use crate::r#move::Move;
use crate::types::Color;
use crate::piece::{piece_value, PieceType, Piece};
use std::io::{self, BufRead, BufReader, Write};
use std::time::{Duration, Instant};
use std::fs::{File, OpenOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};  // やねうら王方式のstopフラグ
use std::thread;


// ============================================================
//  ログ出力
// ============================================================
fn log(msg: &str) {
    // Windows対応: カレントディレクトリにログ出力
    let log_path = if cfg!(windows) {
        "tmoq_nano.log"  // Windowsはカレントディレクトリ
    } else {
        "/tmp/tmoq_nano.log"  // Linux/Mac
    };
    
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(f, "{}", msg);
    }
}

// ============================================================
//  評価値の定数
// ============================================================
const INF:  i32 = 32001;
const MATE: i32 = 32000;
const PAWN_VALUE: i32 = 90;

fn mate_in(ply: i32)   -> i32 { MATE - ply }
fn mated_in(ply: i32)  -> i32 { -MATE + ply }

// ============================================================
//  Null Move Pruning Parameters (やねうら王2016-late準拠)
// ============================================================
const NULL_MOVE_DYNAMIC_ALPHA: i32 = 823;
const NULL_MOVE_DYNAMIC_BETA: i32 = 53;   // 2016-mid: 67 → 2016-late: 53
const NULL_MOVE_RETURN_DEPTH: i32 = 12;

// ============================================================
//  Pruning margins
// ============================================================
fn razor_margin(depth: i32) -> i32 { 512 + 16 * depth }
fn futility_margin(depth: i32) -> i32 { depth * 90 }

// ============================================================
//  Move Count Based Futility
// ============================================================
struct FutilityMoveCounts {
    table: [[i32; 16]; 2],
}

impl FutilityMoveCounts {
    fn new() -> Self {
        let mut fmc = FutilityMoveCounts { table: [[0; 16]; 2] };
        for d in 0..16 {
            let d_f = d as f64;
            fmc.table[0][d] = (2.4 + 0.773 * (d_f + 0.00).powf(1.8)) as i32;
            fmc.table[1][d] = (2.9 + 1.045 * (d_f + 0.49).powf(1.8)) as i32;
        }
        fmc
    }

    fn get(&self, improving: bool, depth: i32) -> i32 {
        let imp = if improving { 1 } else { 0 };
        let d = (depth as usize).min(15);
        self.table[imp][d]
    }
}

// ============================================================
//  LMR reduction table
// ============================================================
const LMR_DEPTH: usize = 64;
const LMR_MC:    usize = 64;

struct ReductionTable {
    table: [[[[i32; LMR_MC]; LMR_DEPTH]; 2]; 2],
}

impl ReductionTable {
    fn new() -> Self {
        let mut rt = ReductionTable { table: [[[[0; LMR_MC]; LMR_DEPTH]; 2]; 2] };
        let k: [[f64; 2]; 2] = [[0.799, 2.281], [0.484, 3.023]];
        for pv in 0..2 {
            for imp in 0..2 {
                for d in 1..LMR_DEPTH {
                    for mc in 1..LMR_MC {
                        let r = k[pv][0] + (d as f64).ln() * (mc as f64).ln() / k[pv][1];
                        if r >= 1.5 {
                            rt.table[pv][imp][d][mc] = r as i32;
                        }
                        if pv == 0 && imp == 0 && rt.table[pv][imp][d][mc] >= 2 {
                            rt.table[pv][imp][d][mc] += 1;
                        }
                    }
                }
            }
        }
        rt
    }

    fn get(&self, is_pv: bool, improving: bool, depth: i32, move_count: i32) -> i32 {
        let pv  = if is_pv { 0 } else { 1 };
        let imp = if improving { 1 } else { 0 };
        let d  = (depth as usize).min(LMR_DEPTH - 1);
        let mc = (move_count as usize).min(LMR_MC - 1);
        self.table[pv][imp][d][mc]
    }
}

// ============================================================
//  置換表（スレッド間共有）
// ============================================================
#[derive(Clone, Copy, PartialEq)]
enum Bound { None, Upper, Lower, Exact }

#[derive(Clone, Copy)]
struct TTEntry {
    key:   u64,
    depth: i8,
    score: i32,
    eval:  i32,
    mv:    u32,
    bound: Bound,
    gen:   u8,
}

const TT_BITS: usize = 20;
const TT_SIZE: usize = 1 << TT_BITS;
const NO_EVAL: i32   = i32::MIN;

struct TT {
    table: Vec<TTEntry>,
    gen:   u8,
}

impl TT {
    fn new() -> Self {
        let empty = TTEntry { key: 0, depth: -99, score: 0, eval: NO_EVAL, mv: 0, bound: Bound::None, gen: 0 };
        TT { table: vec![empty; TT_SIZE], gen: 0 }
    }

    fn clear(&mut self) {
        for e in &mut self.table { e.bound = Bound::None; e.key = 0; e.gen = self.gen; }
    }

    fn new_search(&mut self) { self.gen = self.gen.wrapping_add(1); }

    fn store(&mut self, key: u64, depth: i8, score: i32, eval: i32, mv: Option<Move>, bound: Bound) {
        let idx = (key as usize) & (TT_SIZE - 1);
        let e = &self.table[idx];
        let replace = e.key != key || e.gen != self.gen || bound == Bound::Exact || depth >= e.depth;
        if replace {
            let mv_raw = mv.map(|m| m.raw()).unwrap_or_else(|| if e.key == key { e.mv } else { 0 });
            self.table[idx] = TTEntry { key, depth, score, eval, mv: mv_raw, bound, gen: self.gen };
        }
    }

    fn probe(&self, key: u64) -> Option<TTEntry> {
        let idx = (key as usize) & (TT_SIZE - 1);
        let e = self.table[idx];
        if e.key == key && e.bound != Bound::None { Some(e) } else { None }
    }
}

fn pos_key(pos: &Position) -> u64 {
    let sfen = pos.to_sfen();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in sfen.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ============================================================
//  History / Counter Move テーブル
// ============================================================
const HIST_PT: usize = 15;
const HIST_SQ: usize = 81;

struct HistoryTable {
    table: Box<[[i32; HIST_SQ]; HIST_PT]>,
}

impl HistoryTable {
    fn new() -> Self { HistoryTable { table: Box::new([[0; HIST_SQ]; HIST_PT]) } }
    fn clear(&mut self) {
        for row in self.table.iter_mut() { row.fill(0); }
    }
    fn get(&self, pt: usize, sq: usize) -> i32 {
        if pt < HIST_PT && sq < HIST_SQ { self.table[pt][sq] } else { 0 }
    }
    fn update(&mut self, pt: usize, sq: usize, bonus: i32) {
        if pt < HIST_PT && sq < HIST_SQ {
            let v = &mut self.table[pt][sq];
            *v = (*v + bonus).clamp(-30000, 30000);
        }
    }
}

struct CounterMoveTable {
    table: Box<[[u32; HIST_SQ]; HIST_PT]>,
}

impl CounterMoveTable {
    fn new() -> Self { CounterMoveTable { table: Box::new([[0; HIST_SQ]; HIST_PT]) } }
    fn clear(&mut self) {
        for row in self.table.iter_mut() { row.fill(0); }
    }
    fn get(&self, pt: usize, sq: usize) -> Option<Move> {
        if pt < HIST_PT && sq < HIST_SQ { Move::from_raw(self.table[pt][sq]) } else { None }
    }
    fn set(&mut self, pt: usize, sq: usize, mv: Move) {
        if pt < HIST_PT && sq < HIST_SQ { self.table[pt][sq] = mv.raw(); }
    }
}

#[derive(Clone, Copy, Default)]
struct Stack {
    killers:     [u32; 2],
    static_eval: i32,
	excluded_move: Option<Move>,

}

// ============================================================
//  指し手オーダリング
// ============================================================
fn order_score(
    pos: &Position, mv: Move, tt_mv: Option<Move>, killer: [u32; 2],
    counter_mv: Option<Move>, history: &HistoryTable,
) -> i32 {
    if let Some(t) = tt_mv {
        if mv.raw() == t.raw() { return 3_000_000; }
    }
    if mv.is_capture() {
        let victim = piece_value(pos.piece_at(mv.to()).piece_type());
        let attacker = if mv.is_drop() {
            piece_value(mv.piece_type_dropped())
        } else if let Some(from) = mv.from() {
            piece_value(pos.piece_at(from).piece_type())
        } else { 0 };
        return 2_000_000 + victim * 16 - attacker;
    }
    if mv.is_promotion() && !mv.is_capture() {
        // 取らずに成る手（静かな手）
        if let Some(from) = mv.from() {
            let pt = pos.piece_at(from).piece_type();
            let promo_gain = pt.promote().map(|p| piece_value(p) - piece_value(pt)).unwrap_or(0);
            if promo_gain > 0 { 
                // 取らない成りは killer よりは低く、drop よりは高く
                return 500_000 + promo_gain; 
            }
        }
    }
    if mv.raw() == killer[0] { return 900_000; }
    if mv.raw() == killer[1] { return 800_000; }
    if let Some(cm) = counter_mv {
        if mv.raw() == cm.raw() { return 700_000; }
    }
    if mv.is_drop() { return 200 + piece_value(mv.piece_type_dropped()); }
    if let Some(from) = mv.from() {
        let pt = pos.piece_at(from).piece_type() as usize;
        let sq = mv.to().0 as usize;
        return history.get(pt, sq);
    }
    0
}

// ============================================================
//  定跡
// ============================================================
struct BookMove {
    mv:    String,
    score: i32,
    count: u32,
}

struct OpeningBook {
    table: HashMap<String, Vec<BookMove>>,
}

impl OpeningBook {
    fn new() -> Self { OpeningBook { table: HashMap::new() } }

    fn load<P: AsRef<Path>>(path: P) -> Self {
        let mut book = OpeningBook::new();
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => {
                log(&format!("book: file not found: {}", path.as_ref().display()));
                return book;
            }
        };
        let reader = BufReader::new(file);
        let mut current_key = String::new();
        let mut count = 0u32;

        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            let line = line.trim().to_string();
            if line.is_empty() || line.starts_with('#') { continue; }

            if line.starts_with("sfen ") {
                let sfen_body = &line[5..];
                let parts: Vec<&str> = sfen_body.splitn(4, ' ').collect();
                current_key = if parts.len() >= 3 {
                    format!("{} {} {}", parts[0], parts[1], parts[2])
                } else {
                    sfen_body.to_string()
                };
            } else if !current_key.is_empty() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let mv_str    = parts[0].to_string();
                    let score: i32 = parts[2].parse().unwrap_or(0);
                    let c: u32     = parts[4].parse().unwrap_or(1);
                    book.table.entry(current_key.clone())
                        .or_insert_with(Vec::new)
                        .push(BookMove { mv: mv_str, score, count: c });
                    count += 1;
                }
            }
        }
        log(&format!("book: loaded {} positions, {} moves", book.table.len(), count));
        book
    }

    fn probe(&self, pos: &Position) -> Option<String> {
        let sfen = pos.to_sfen();
        let parts: Vec<&str> = sfen.splitn(4, ' ').collect();
        if parts.len() < 3 { return None; }
        let key = format!("{} {} {}", parts[0], parts[1], parts[2]);
        let moves = self.table.get(&key)?;
        if moves.is_empty() { return None; }
        let best = moves.iter().max_by(|a, b| {
            a.count.cmp(&b.count).then(a.score.cmp(&b.score))
        })?;
        Some(best.mv.clone())
    }
}

// ============================================================
//  探索エンジン（スレッドローカル）
// ============================================================
struct SearchWorker {
    tt:         Arc<Mutex<TT>>,  // スレッド間共有
    history:    HistoryTable,
    counter:    CounterMoveTable,
    lmr:        ReductionTable,
    fmc:        FutilityMoveCounts,
    nodes:      u64,
    stop:       Arc<AtomicBool>,  // やねうら王方式: atomic停止フラグ
}

impl SearchWorker {
    fn new(tt: Arc<Mutex<TT>>, stop: Arc<AtomicBool>) -> Self {
        SearchWorker {
            tt,
            history:    HistoryTable::new(),
            counter:    CounterMoveTable::new(),
            lmr:        ReductionTable::new(),
            fmc:        FutilityMoveCounts::new(),
            nodes:      0,
            stop,
        }
    }

    fn time_over(&self) -> bool {
        // やねうら王方式: atomic loadで高速チェック
        self.stop.load(Ordering::Relaxed)
    }

    fn material(&self, pos: &Position) -> i32 {
        let raw = pos.evaluate_improved();  // 改良版評価関数を使用
        match pos.side_to_move() {
            Color::Black =>  raw,
            Color::White => -raw,
        }
    }

    fn qsearch(
        &mut self, pos: &mut Position, stack: &mut Vec<Stack>,
        ply: i32, mut alpha: i32, beta: i32,
    ) -> i32 {
        self.nodes += 1;
        
        // やねうら王方式: 毎ノード時間チェック
        if self.time_over() {
            return alpha;
        }
        
        if ply >= 16 { return self.material(pos); }

        let in_check = pos.in_check();
        let key = pos_key(pos);
        
        let tte = {
            let tt = self.tt.lock().unwrap();
            tt.probe(key)
        };

        let stand_pat: i32;
        if in_check {
            stand_pat = -INF;
        } else {
            if let Some(e) = tte {
                stand_pat = if e.eval != NO_EVAL { e.eval } else { self.material(pos) };
                let tt_score = e.score;
                if e.depth >= 0 {
                    match e.bound {
                        Bound::Exact => return tt_score,
                        Bound::Lower => if tt_score >= beta  { return tt_score; },
                        Bound::Upper => if tt_score <= alpha { return tt_score; },
                        Bound::None  => {}
                    }
                }
            } else {
                stand_pat = self.material(pos);
                let mut tt = self.tt.lock().unwrap();
                tt.store(key, -1, 0, stand_pat, None, Bound::None);
            }

            if stand_pat >= beta { return stand_pat; }
            if stand_pat > alpha { alpha = stand_pat; }
        }

        let moves = pos.generate_legal_moves();
        let mut tactical: Vec<Move> = if in_check {
            moves
        } else {
            // qsearch では「取る手」と「取りながら成る手」のみ生成
            // 「只で成る手」は静かな手なので qsearch では生成しない
            moves.into_iter().filter(|m| {
                if m.is_capture() {
                    true  // 取る手は全て生成
                } else if m.is_promotion() {
                    false  // 取らずに成る手は生成しない（静かな手扱い）
                } else {
                    false
                }
            }).collect()
        };

        tactical.sort_unstable_by_key(|mv| {
            if mv.is_capture() {
                let victim   = piece_value(pos.piece_at(mv.to()).piece_type());
                let attacker = mv.from().map(|f| piece_value(pos.piece_at(f).piece_type())).unwrap_or(0);
                -(victim * 16 - attacker)
            } else if mv.is_promotion() { -500_000i32 } else { 0i32 }
        });

        let mut best_score = if in_check { -INF } else { stand_pat };
        let mut best_mv: Option<Move> = None;

        for mv in &tactical {
            let mut child = pos.clone();
            child.do_move(*mv);

            if stack.len() <= ply as usize + 1 {
                stack.resize(ply as usize + 2, Stack::default());
            }

            let score = -self.qsearch(&mut child, stack, ply + 1, -beta, -alpha);
            if self.time_over() { return alpha; }

            if score > best_score {
                best_score = score;
                best_mv    = Some(*mv);
            }
            if score >= beta {
                let mut tt = self.tt.lock().unwrap();
                tt.store(key, 0, score, stand_pat, best_mv, Bound::Lower);
                return score;
            }
            if score > alpha { alpha = score; }
        }

        if in_check && tactical.is_empty() {
            let val = mated_in(ply);
            let mut tt = self.tt.lock().unwrap();
            tt.store(key, 30, val, NO_EVAL, None, Bound::Exact);
            return val;
        }

        let bound = if best_score <= alpha { Bound::Upper } else { Bound::Exact };
        let mut tt = self.tt.lock().unwrap();
        tt.store(key, 0, best_score, if in_check { NO_EVAL } else { stand_pat }, best_mv, bound);
        best_score
    }

    fn search(
        &mut self, pos: &mut Position, stack: &mut Vec<Stack>,
        ply: i32, depth: i32, alpha: i32, beta: i32, is_pv: bool,
    ) -> i32 {
        self.nodes += 1;
        
        // やねうら王方式: 毎ノード時間チェック（atomic loadは高速）
        if self.time_over() {
            return alpha;
        }

        if depth <= 0 {
            return self.qsearch(pos, stack, ply, alpha, beta);
        }

        let alpha = {
            let a = alpha.max(mated_in(ply));
            let b = beta.min(mate_in(ply + 1));
            if a >= b { return a; }
            a
        };
        let mut alpha = alpha;

        let key = pos_key(pos);
        let tte = {
            let tt = self.tt.lock().unwrap();
            tt.probe(key)
        };
        let tt_mv: Option<Move> = tte.and_then(|e| Move::from_raw(e.mv));

		let tt_score = tte.map(|e| e.score).unwrap_or(0);
		let tt_depth = tte.map(|e| e.depth).unwrap_or(0);
		
        // --- 1手詰め判定 (PV node、王手なし、depth >= 4 のとき) ---
        // Classicに合わせて条件を厳格化：不要な呼び出しを削減
        if is_pv && !pos.in_check() && depth >= 4 {
            if let Some(mate_mv) = pos.mate1ply() {
                let mate_score = mate_in(ply);
                let mut tt = self.tt.lock().unwrap();
                tt.store(key, depth as i8, mate_score, NO_EVAL, Some(mate_mv), Bound::Exact);
                return mate_score;
            }
        }

        if !is_pv {
            if let Some(e) = tte {
                if e.depth >= depth as i8 {
                    match e.bound {
                        Bound::Exact => return e.score,
                        Bound::Lower => if e.score >= beta  { return e.score; },
                        Bound::Upper => if e.score <= alpha { return e.score; },
                        Bound::None  => {}
                    }
                }
            }
        }

        let in_check = pos.in_check();
		
		// Singular Extension判定
		let mut extension = 0;
		
		let singular_node = !is_pv 
			&& depth >= 8 
			&& tt_mv.is_some() 
			&& tt_depth >= (depth - 3) as i8 
			&& !in_check;
		
		if singular_node {
			let tt_move = tt_mv.unwrap();
			let singular_beta = tt_score - (16 * depth) / 8;
			
			// excludedMoveを設定
			if (ply as usize) < stack.len() {
				stack[ply as usize].excluded_move = Some(tt_move);
			}
			
			// ttMove以外を浅く探索
			let singular_value = self.search(
				pos, stack, ply, 
				depth / 2,
				singular_beta - 1, 
				singular_beta, 
				false
			);
			
			// excludedMoveをクリア
			if (ply as usize) < stack.len() {
				stack[ply as usize].excluded_move = None;
			}
			
			// singular判定
			if singular_value < singular_beta {
				extension = 1;
			}
		}

        let eval: i32;
        if in_check {
            eval = -INF;
        } else if let Some(e) = tte {
            eval = if e.eval != NO_EVAL { e.eval } else { self.material(pos) };
        } else {
            eval = self.material(pos);
            let mut tt = self.tt.lock().unwrap();
            tt.store(key, -1, 0, eval, None, Bound::None);
        }

        while stack.len() <= ply as usize + 2 {
            stack.push(Stack::default());
        }
        stack[ply as usize].static_eval = eval;

        let improving = if in_check {
            true
        } else if ply >= 2 {
            let prev2 = stack[(ply - 2) as usize].static_eval;
            prev2 == NO_EVAL || eval >= prev2
        } else {
            true
        };
        
        // --- IID (Internal Iterative Deepening) ---
        // TT moveがない時、浅い探索でTT moveを見つける
        let mut tt_mv = tt_mv;  // mutableに変更
        
        if tt_mv.is_none() && depth >= 6 && (is_pv || (!in_check && eval >= beta)) {
            // 浅い探索を実行
            let iid_depth = if is_pv { depth / 2 } else { depth / 2 - 1 };
            
            if iid_depth > 0 {
                let _iid_score = self.search(
                    pos, stack, ply,
                    iid_depth,
                    alpha, beta,
                    is_pv
                );
                
                // TTに記録された手を取得
                let tte_after_iid = {
                    let tt = self.tt.lock().unwrap();
                    tt.probe(key)
                };
                
                tt_mv = tte_after_iid.and_then(|e| Move::from_raw(e.mv));
            }
        }

        // Razoring
        if !is_pv && !in_check && depth < 4 && eval + razor_margin(depth) <= alpha && tt_mv.is_none() {
            if depth <= 1 && eval + razor_margin(3) <= alpha {
                return self.qsearch(pos, stack, ply, alpha, beta);
            }
            let ralpha = alpha - razor_margin(depth);
            let mut child = pos.clone();
            let v = self.qsearch(&mut child, stack, ply, ralpha, ralpha + 1);
            if v <= ralpha { return v; }
        }

        // Futility Pruning
        if !is_pv && !in_check && depth < 7 && eval - futility_margin(depth) >= beta && eval < MATE - 300 {
            return eval - futility_margin(depth);
        }

        // Null Move Pruning（やねうら王2016-late準拠）
        if !is_pv && !in_check && depth >= 2 && eval >= beta {
            // やねうら王2016-late: BETA = 53
            let r = ((NULL_MOVE_DYNAMIC_ALPHA + NULL_MOVE_DYNAMIC_BETA * depth) / 256 
                     + ((eval - beta) / PAWN_VALUE).min(3)).max(1);
            let mut null_pos = pos.make_null_move();
            
            let null_value = if depth - r <= 0 {
                -self.qsearch(&mut null_pos, stack, ply + 1, -beta, -beta + 1)
            } else {
                -self.search(&mut null_pos, stack, ply + 1, depth - r, -beta, -beta + 1, false)
            };

            if null_value >= beta {
                if null_value >= MATE - 300 {
                    // do nothing
                } else if depth < NULL_MOVE_RETURN_DEPTH && beta.abs() < MATE - 300 {
                    return null_value;
                } else {
                    let v = if depth - r <= 0 {
                        self.qsearch(pos, stack, ply, beta - 1, beta)
                    } else {
                        self.search(pos, stack, ply, depth - r, beta - 1, beta, false)
                    };
                    if v >= beta { return null_value; }
                }
            }
        }

        let moves = pos.generate_legal_moves();
        if moves.is_empty() {
            return mated_in(ply);
        }

        let killer = stack[ply as usize].killers;
        let counter_mv = None;

        let mut scored: Vec<(Move, i32)> = moves.iter()
            .map(|&mv| (mv, order_score(pos, mv, tt_mv, killer, counter_mv, &self.history)))
            .collect();
        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        let orig_alpha  = alpha;
        let mut best_score = -INF;
        let mut best_mv:    Option<Move> = None;
        let mut move_count  = 0;
        let mut quiets: Vec<Move> = Vec::with_capacity(32);

        let futility_move_count = self.fmc.get(improving, depth);

        for (mv, _) in &scored {
            // Singular Extension: excluded_moveはスキップ
            if (ply as usize) < stack.len() {
                if let Some(excluded) = stack[ply as usize].excluded_move {
                    if mv.raw() == excluded.raw() {
                        continue;
                    }
                }
            }
            
            let capture_or_promo = mv.is_capture() || mv.is_promotion();
            move_count += 1;

            if !is_pv && !in_check && best_score > -MATE + 300
                && move_count >= futility_move_count && !capture_or_promo
            {
                continue;
            }
            
            // --- History Pruning (やねうら王2016-late方式・保守的版) ---
            // LMR後の深さが9未満で、historyの値が非常に悪い手を枝刈り
            if !is_pv && !in_check && !capture_or_promo && move_count >= 2 {
                // LMR後の深さを計算
                let reduction = if depth >= 3 && move_count > 1 {
                    self.lmr.get(is_pv, improving, depth, move_count)
                } else {
                    0
                };
                let lmr_depth = (depth - 1 - reduction).max(0);
                
                // やねうら王2016-late: lmrDepth < 9 で適用
                if lmr_depth < 9 {
                    let history_value = if let Some(from) = mv.from() {
                        let pt = pos.piece_at(from).piece_type() as usize;
                        let sq = mv.to().0 as usize;
                        self.history.get(pt, sq)
                    } else {
                        0
                    };
                    
                    // より保守的な閾値: -8000（やねうら王のREDUCTION_BY_HISTORYを参考）
                    // 元の実装: -20000 * depth^2
                    // 新実装: -8000（固定閾値、保守的）
                    if history_value < -8000 {
                        continue;
                    }
                }
            }

            // --- Check Extension (王手の指し手を延長) ---
            let gives_check = {
                let mut test_pos = pos.clone();
                test_pos.do_move(*mv);
                test_pos.in_check()
            };
            
            // check_extension: 王手なら1手延長
            let check_extension = if gives_check { 1 } else { 0 };
            
            // total extension = singular + check
            let total_extension = extension + check_extension;
            let new_depth = depth - 1 + total_extension;

            let mut child = pos.clone();
            child.do_move(*mv);

            let score = if depth >= 3 && move_count > 1 && !capture_or_promo {
                let r = self.lmr.get(is_pv, improving, depth, move_count);
                let d = (new_depth - r).max(1);
                let s = -self.search(&mut child, stack, ply + 1, d, -(alpha + 1), -alpha, false);

                if s > alpha && r > 0 {
                    -self.search(&mut child, stack, ply + 1, new_depth, -(alpha + 1), -alpha, false)
                } else {
                    s
                }
            } else if move_count == 1 {
                -self.search(&mut child, stack, ply + 1, new_depth, -beta, -alpha, is_pv)
            } else {
                -self.search(&mut child, stack, ply + 1, new_depth, -(alpha + 1), -alpha, false)
            };

            if self.time_over() { return 0; }

            if score > best_score {
                best_score = score;
                best_mv    = Some(*mv);
            }

            if score > alpha {
                alpha = score;

                if alpha >= beta {
                    if !capture_or_promo {
                        let k = &mut stack[ply as usize].killers;
                        if k[0] != mv.raw() {
                            k[1] = k[0];
                            k[0] = mv.raw();
                        }
                        let bonus = (depth * depth + depth + 1).min(1000);
                        if let Some(from) = mv.from() {
                            let pt = pos.piece_at(from).piece_type() as usize;
                            let sq = mv.to().0 as usize;
                            self.history.update(pt, sq, bonus);
                            self.counter.set(pt, sq, *mv);
                        }
                        for &q in &quiets {
                            if let Some(from) = q.from() {
                                let pt = pos.piece_at(from).piece_type() as usize;
                                let sq = q.to().0 as usize;
                                self.history.update(pt, sq, -bonus);
                            }
                        }
                    }

                    let mut tt = self.tt.lock().unwrap();
                    tt.store(key, depth as i8, score, eval, best_mv, Bound::Lower);
                    return score;
                }
            }

            if !capture_or_promo && quiets.len() < 64 {
                quiets.push(*mv);
            }
        }

        let bound = if best_score <= orig_alpha { Bound::Upper } else { Bound::Exact };
        let mut tt = self.tt.lock().unwrap();
        tt.store(key, depth as i8, best_score, eval, best_mv, bound);
        best_score
    }
}

// ============================================================
//  スレッド結果
// ============================================================
struct ThreadResult {
    best_move:      Move,
    best_score:     i32,
    completed_depth: i32,
    nodes:          u64,
}

// ============================================================
//  マルチスレッド探索
// ============================================================
fn search_with_threads(
    pos: &Position,
    time_ms: u64,
    max_depth: i32,
    num_threads: usize,
) -> Option<Move> {
    let tt = Arc::new(Mutex::new(TT::new()));
    {
        let mut t = tt.lock().unwrap();
        t.new_search();
    }

    // やねうら王方式: Atomic停止フラグ + タイマースレッド
    let stop = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let time_limit = Duration::from_millis(time_ms.saturating_sub(50).max(10));
    
    log(&format!("Search with time limit: {}ms", time_ms));

    // タイマースレッド（やねうら王方式）
    let stop_clone = Arc::clone(&stop);
    let timer_handle = thread::spawn(move || {
        thread::sleep(time_limit);
        stop_clone.store(true, Ordering::Relaxed);
    });

    let root_moves = pos.generate_legal_moves();
    if root_moves.is_empty() {
        stop.store(true, Ordering::Relaxed);
        let _ = timer_handle.join();
        log("No legal moves");
        return None;
    }

    // 各スレッドを起動
    let mut handles = vec![];
    let results = Arc::new(Mutex::new(Vec::<ThreadResult>::new()));

    for thread_id in 0..num_threads {
        let pos_clone = pos.clone();
        let root_moves_clone = root_moves.clone();
        let tt_clone = Arc::clone(&tt);
        let stop_clone = Arc::clone(&stop);
        let results_clone = Arc::clone(&results);
        let start_clone = start;

        let handle = thread::spawn(move || {
            let mut worker = SearchWorker::new(tt_clone, stop_clone);
            let mut stack = vec![Stack::default(); 64];
            let mut best_move = root_moves_clone[0];
            let mut best_score = -INF;
            let mut completed_depth = 0;

            // Lazy SMP: 全スレッドが depth 1 から開始
            let _depth_offset = (thread_id % 4) as i32;

            for depth in 1..=max_depth {
                // 深化前の時間チェック
                if worker.time_over() {
                    log(&format!("Thread {}: Time over before depth {}", thread_id, depth));
                    break;
                }

                let mut scored: Vec<(Move, i32)> = root_moves_clone.iter()
                    .map(|&mv| (mv, order_score(&pos_clone, mv, Some(best_move), [0, 0], None, &worker.history)))
                    .collect();
                scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

                let mut iter_best  = best_move;
                let mut iter_score = -INF;
                let mut alpha      = -INF;

                for (mv, _) in &scored {
                    if worker.time_over() {
                        log(&format!("Thread {}: Time over during depth {}", thread_id, depth));
                        break;
                    }

                    let mut child = pos_clone.clone();
                    child.do_move(*mv);

                    let score = if alpha == -INF {
                        -worker.search(&mut child, &mut stack, 1, depth - 1, -INF, INF, true)
                    } else {
                        let s = -worker.search(&mut child, &mut stack, 1, depth - 1, -(alpha + 1), -alpha, false);
                        if s > alpha {
                            -worker.search(&mut child, &mut stack, 1, depth - 1, -INF, -alpha, true)
                        } else { s }
                    };

                    if worker.time_over() { break; }

                    if score > iter_score {
                        iter_score = score;
                        iter_best  = *mv;
                    }
                    if score > alpha { alpha = score; }
                }

                if !worker.time_over() {
                    best_move  = iter_best;
                    best_score = iter_score;
                    completed_depth = depth;

                    // メインスレッド（thread_id == 0）のみ info を出力
                    if thread_id == 0 {
                        let elapsed = start_clone.elapsed().as_millis();
                        let out = io::stdout();
                        let mut out = out.lock();
                        
                        // 詰みスコアか通常スコアかを判定
                        let score_str = if best_score > MATE - 300 {
                            // 詰みスコア（正の方向）
                            let ply_to_mate = (MATE - best_score + 1) / 2;
                            format!("mate {}", ply_to_mate)
                        } else if best_score < -MATE + 300 {
                            // 詰まされるスコア（負の方向）
                            let ply_to_mated = (MATE + best_score + 1) / 2;
                            format!("mate -{}", ply_to_mated)
                        } else {
                            // 通常の評価値
                            format!("cp {}", best_score)
                        };
                        
                        let _ = writeln!(out, "info depth {} score {} nodes {} time {} pv {}",
                            depth, score_str, worker.nodes, elapsed, best_move.to_usi());
                        let _ = out.flush();
                    }
                }

                if best_score > MATE - 300 || best_score < -MATE + 300 { break; }
            }

            // 結果を保存
            let mut res = results_clone.lock().unwrap();
            res.push(ThreadResult {
                best_move,
                best_score,
                completed_depth,
                nodes: worker.nodes,
            });
        });

        handles.push(handle);
    }

    // 全スレッド終了待ち
    for handle in handles {
        let _ = handle.join();
    }

    // タイマースレッド終了（やねうら王方式）
    stop.store(true, Ordering::Relaxed);
    let _ = timer_handle.join();

    let elapsed = start.elapsed().as_millis();
    log(&format!("Search completed in {}ms", elapsed));

    // 最も深く探索したスレッドの結果を採用
    let results = results.lock().unwrap();
    if results.is_empty() {
        log("No results, using first root move");
        return Some(root_moves[0]);
    }

    let best_result = results.iter().max_by_key(|r| r.completed_depth).unwrap();
    
    log(&format!("bestmove {} (score={}, depth={}, nodes={})",
        best_result.best_move.to_usi(), best_result.best_score,
        best_result.completed_depth, best_result.nodes));

    Some(best_result.best_move)
}

// ============================================================
//  USIメインループ
// ============================================================
pub fn usi_loop() {
    use std::io::Write;
    
    let stdin  = io::stdin();
    let stdout = io::stdout();

    // Windows対策: stdoutのバッファリングを無効化
    // C言語の setvbuf(_IONBF) 相当の処理
    // Rustでは明示的なflushで対応
    
    let mut pos         = Position::start_position();
    let mut initialized = false;
    let mut book_path   = "book/yaneuraoh.db".to_string();
    let mut book        = OpeningBook::new();
    let mut num_threads = 1usize;

    log("=== TMOQ_wcsc36 engine started ===");

    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let line = line.trim().to_string();
        if line.is_empty() { continue; }
        log(&format!("< {}", line));

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() { continue; }

        match tokens[0] {
            "usi" => {
                let mut out = stdout.lock();
                writeln!(out, "id name TMOQ_wcsc36").unwrap();
                writeln!(out, "id author TMOQ").unwrap();
                writeln!(out, "option name BookFile type string default book/yaneuraoh.db").unwrap();
                writeln!(out, "option name OwnBook type check default true").unwrap();
                writeln!(out, "option name Threads type spin default 1 min 1 max 8").unwrap();
                writeln!(out, "usiok").unwrap();
                out.flush().unwrap();
            }

            "setoption" => {
                let mut i = 1;
                while i < tokens.len() {
                    if tokens[i] == "name" && i + 3 < tokens.len() {
                        if tokens[i+1] == "BookFile" && tokens[i+2] == "value" {
                            book_path = tokens[i+3].to_string();
                            log(&format!("setoption BookFile = {}", book_path));
                            i += 4;
                        } else if tokens[i+1] == "Threads" && tokens[i+2] == "value" {
                            if let Ok(t) = tokens[i+3].parse::<usize>() {
                                num_threads = t.clamp(1, 8);
                                log(&format!("setoption Threads = {}", num_threads));
                            }
                            i += 4;
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
            }

            "isready" => {
                if !initialized {
                    crate::attacks::init_attack_tables();
                    book = OpeningBook::load(&book_path);
                    initialized = true;
                }
                let mut out = stdout.lock();
                writeln!(out, "readyok").unwrap();
                out.flush().unwrap();
            }

            "usinewgame" => {
                pos = Position::start_position();
            }

            "position" => {
                pos = parse_position(&tokens);
                log(&format!("  sfen: {}", pos.to_sfen()));
            }

            "go" => {
                let (time_ms, max_depth) = parse_go(&tokens, pos.side_to_move());
                log(&format!("  go: time={}ms depth={} threads={}", time_ms, max_depth, num_threads));

                // 定跡チェック
                if let Some(book_mv_str) = book.probe(&pos) {
                    let legal = pos.generate_legal_moves();
                    if let Some(mv) = legal.iter().find(|m| m.to_usi() == book_mv_str) {
                        log(&format!("book hit: {}", book_mv_str));
                        {
                            let mut out = stdout.lock();
                            writeln!(out, "info string book move {}", book_mv_str).unwrap();
                            out.flush().unwrap();
                        }
                        // bestmove を確実に送信（Windows対策）
                        {
                            use std::io::Write;
                            let mut out = std::io::stdout();
                            writeln!(out, "bestmove {}", mv.to_usi()).unwrap();
                            out.flush().unwrap();
                        }
                        continue;
                    }
                }

                let result = search_with_threads(&pos, time_ms, max_depth, num_threads);

                // bestmove を確実に送信（Windows対策）
                {
                    use std::io::Write;
                    let mut out = std::io::stdout();
                    match result {
                        Some(mv) => {
                            writeln!(out, "bestmove {}", mv.to_usi()).unwrap();
                            out.flush().unwrap();
                            log(&format!("> bestmove {}", mv.to_usi()));
                        }
                        None => {
                            writeln!(out, "bestmove resign").unwrap();
                            out.flush().unwrap();
                            log("> bestmove resign");
                        }
                    }
                }
            }

            "stop" => {}

            "quit" => { log("quit"); break; }

            _ => {}
        }
    }
}

fn parse_position(tokens: &[&str]) -> Position {
    let mut pos = Position::start_position();
    let mut i = 1;

    if i >= tokens.len() { return pos; }

    match tokens[i] {
        "startpos" => { i += 1; }
        "sfen" => {
            i += 1;
            let mut parts = Vec::new();
            while i < tokens.len() && tokens[i] != "moves" {
                parts.push(tokens[i]);
                i += 1;
            }
            let sfen = parts.join(" ");
            if let Some(p) = Position::from_sfen(&sfen) {
                pos = p;
            } else {
                log(&format!("  sfen parse error: {}", sfen));
            }
        }
        _ => {}
    }

    if i < tokens.len() && tokens[i] == "moves" {
        i += 1;
        while i < tokens.len() {
            if let Some(mv) = Move::from_usi(tokens[i]) {
                pos.do_move(mv);
            } else {
                log(&format!("  move parse error: {}", tokens[i]));
            }
            i += 1;
        }
    }

    pos
}

fn parse_go(tokens: &[&str], side: Color) -> (u64, i32) {
    let mut time_ms:   u64 = 5_000;
    let mut inc_ms:    u64 = 0;     // フィッシャールール加算時間
    let mut byoyomi:   u64 = 0;
    let mut max_depth: i32 = 64;

    let mut i = 1;
    while i < tokens.len() {
        match tokens[i] {
            "btime" if side == Color::Black => {
                if let Some(t) = tokens.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                    time_ms = t;  // ミリ秒でそのまま使用
                    i += 1;
                }
            }
            "wtime" if side == Color::White => {
                if let Some(t) = tokens.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                    time_ms = t;  // ミリ秒でそのまま使用
                    i += 1;
                }
            }
            "binc" if side == Color::Black => {
                if let Some(t) = tokens.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                    inc_ms = t;
                    i += 1;
                }
            }
            "winc" if side == Color::White => {
                if let Some(t) = tokens.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                    inc_ms = t;
                    i += 1;
                }
            }
            "byoyomi" => {
                if let Some(t) = tokens.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                    byoyomi = t;
                    i += 1;
                }
            }
            "movetime" => {
                if let Some(t) = tokens.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                    time_ms = t;
                    i += 1;
                }
            }
            "depth" => {
                if let Some(d) = tokens.get(i+1).and_then(|s| s.parse::<i32>().ok()) {
                    max_depth = d;
                    time_ms   = 999_999_999;
                    i += 1;
                }
            }
            "infinite" => {
                time_ms   = 999_999_999;
                max_depth = 64;
            }
            _ => {}
        }
        i += 1;
    }

    // 時間配分の計算
    let allocated_time = if byoyomi > 0 {
        // 秒読み: 秒読み時間の70%を使う（安全マージン30%）
        (byoyomi as f64 * 0.7) as u64
    } else if inc_ms > 0 {
        // フィッシャールール: より保守的に
        let base_time = time_ms / 60;  // 残り時間を60手で使う
        let inc_bonus = (inc_ms as f64 * 0.5) as u64;
        let mut allocated = base_time + inc_bonus;
        
        // 残り時間が少ない場合はさらに保守的に
        if time_ms == 0 {
            // 残り時間ゼロ: 超保守的（加算時間の25%、最低200ms）
            // 通信遅延を考慮して、加算時間の大部分は残す
            allocated = (inc_ms as f64 * 0.25).max(200.0) as u64;
            log(&format!("ZERO time remaining: using only 25% of increment ({}ms)", allocated));
        } else if time_ms < 1000 {
            // 残り1秒未満: 超保守的（加算時間の30%、最低250ms）
            allocated = (inc_ms as f64 * 0.30).max(250.0) as u64;
            log(&format!("Very critical time (<1s): using only 30% of increment ({}ms)", allocated));
        } else if time_ms < 5000 {
            // 残り5秒未満: 非常に保守的（加算時間の35%、最低300ms）
            allocated = (inc_ms as f64 * 0.35).max(300.0) as u64;
            log(&format!("Critical time (<5s): using only 35% of increment ({}ms)", allocated));
        } else if time_ms < 10000 {
            // 残り10秒未満: 保守的（加算時間の40%、最低300ms）
            allocated = (inc_ms as f64 * 0.4).max(300.0) as u64;
            log(&format!("Low time (<10s): using only 40% of increment ({}ms)", allocated));
        } else if time_ms < 30000 {
            // 残り30秒未満: やや保守的（加算時間の45%、最低500ms）
            allocated = (inc_ms as f64 * 0.45).max(500.0) as u64;
            log(&format!("Medium time (<30s): using 45% of increment ({}ms)", allocated));
        } else if time_ms < 60000 {
            // 残り60秒未満: 通常配分の70%
            allocated = (allocated as f64 * 0.7) as u64;
            log(&format!("Normal time (<60s): using 70% of allocation ({}ms)", allocated));
        }
        
        // 最小200ms、最大15秒に制限
        allocated.max(200).min(15000)
    } else {
        // 通常: 残り時間を50手で使う
        (time_ms / 50).max(500)
    };

    log(&format!("Time allocation: total={}ms, inc={}ms, byoyomi={}ms, allocated={}ms", 
                 time_ms, inc_ms, byoyomi, allocated_time));

    (allocated_time, max_depth)
}
