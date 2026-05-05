// movegen.rs - 合法手生成
//
// やねうら王nanoを参考にした実装。
// 疑似合法手を生成してから pos.legal(move) で王手放置を除外する。

use crate::types::{Color, Square, Rank, File};
use crate::piece::{PieceType, HandPiece};
use crate::bitboard::Bitboard;
use crate::r#move::Move;
use crate::position::Position;
use crate::attacks::attacks_from;

impl Position {
    // =========================================================
    //  公開API: 完全な合法手生成
    //  王手放置になる手を除外した本当の合法手のみ返す
    // =========================================================
    pub fn generate_legal_moves(&self) -> Vec<Move> {
        // 疑似合法手を全部生成してから legal() でフィルタ
        let pseudo = self.generate_pseudo_legal_moves();
        pseudo.into_iter().filter(|mv| self.legal(*mv)).collect()
    }

    // =========================================================
    //  疑似合法手生成（王手放置チェックなし）
    //  盤のルール（二歩・行き所のない駒）は除外済み
    // =========================================================
    pub fn generate_pseudo_legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(200);
        let us = self.side_to_move();
        let occupied = self.occupied_bb();
        let our_pieces = self.color_bb(us);

        // --- 1. 盤上の駒を動かす ---
        for from_sq in our_pieces {
            let piece = self.piece_at(from_sq);
            let pt = piece.piece_type();
            let targets = self.get_attacks(pt, us, from_sq, occupied);
            let dests = targets & !our_pieces; // 自駒のある場所は除外

            for to_sq in dests {
                let cap = self.piece_at(to_sq);
                let cap_pt = if !cap.is_empty() { Some(cap.piece_type()) } else { None };

                let can_promote = pt.can_promote()
                    && (from_sq.can_promote(us) || to_sq.can_promote(us));
                let must_promote = self.must_promote(pt, us, to_sq);

                if must_promote {
                    if can_promote {
                        let mut mv = Move::new_normal(from_sq, to_sq, pt, true);
                        if let Some(c) = cap_pt { mv = mv.with_capture(c); }
                        moves.push(mv);
                    }
                    // 行き所なし・成れないは指せない（何も追加しない）
                } else {
                    // 不成
                    let mut mv = Move::new_normal(from_sq, to_sq, pt, false);
                    if let Some(c) = cap_pt { mv = mv.with_capture(c); }
                    moves.push(mv);
                    // 成り（任意）
                    if can_promote {
                        let mut mv = Move::new_normal(from_sq, to_sq, pt, true);
                        if let Some(c) = cap_pt { mv = mv.with_capture(c); }
                        moves.push(mv);
                    }
                }
            }
        }

        // --- 2. 駒打ち ---
        let hand = self.hand(us);
        let empty_squares = !occupied;

        for hp_idx in 0..7u8 {
            let hp = unsafe { std::mem::transmute::<u8, HandPiece>(hp_idx) };
            if hand.count(hp) == 0 { continue; }
            let pt = hp.to_piece_type();

            for to_sq in empty_squares {
                if self.can_drop(pt, us, to_sq) {
                    moves.push(Move::new_drop(pt, to_sq));
                }
            }
        }

        moves
    }

    // =========================================================
    //  legal(move): 指し手が合法か（王手放置していないか）
    //  do_move して自玉が王手になっていれば非合法
    // =========================================================
    pub fn legal(&self, mv: Move) -> bool {
        let us = self.side_to_move();
        let mut after = self.clone();
        after.do_move(mv);
        // do_move後は相手番になっているので us の玉が取られないかチェック
        !after.is_attacked(us)
    }

    // =========================================================
    //  is_attacked(c): c の玉に相手の利きが届いているか
    //  = c 側が王手されているか
    // =========================================================
    pub fn is_attacked(&self, c: Color) -> bool {
        let king_sq = match self.king_square(c) {
            Some(sq) => sq,
            None => return true, // 玉がない = 取られた = 王手放置
        };
        let them = c.opposite();
        let their_pieces = self.color_bb(them);
        let occupied = self.occupied_bb();

        for sq in their_pieces {
            let pt = self.piece_at(sq).piece_type();
            let attacks = self.get_attacks(pt, them, sq, occupied);
            if attacks.is_set(king_sq) {
                return true;
            }
        }
        false
    }

    // =========================================================
    //  in_check(): 現在の手番側が王手されているか
    // =========================================================
    pub fn in_check(&self) -> bool {
        self.is_attacked(self.side_to_move())
    }

    // =========================================================
    //  内部: 駒の利き取得（障害物考慮）
    // =========================================================
    pub(crate) fn get_attacks(&self, pt: PieceType, c: Color, from: Square, occupied: Bitboard) -> Bitboard {
        match pt {
            PieceType::Pawn | PieceType::Knight | PieceType::Silver
            | PieceType::Gold | PieceType::King
            | PieceType::ProPawn | PieceType::ProLance | PieceType::ProKnight
            | PieceType::ProSilver => {
                attacks_from(pt, c, from)
            }
            PieceType::Lance  => self.lance_attacks(c, from, occupied),
            PieceType::Bishop => self.bishop_attacks(from, occupied),
            PieceType::Rook   => self.rook_attacks(from, occupied),
            PieceType::Horse  => self.bishop_attacks(from, occupied) | attacks_from(PieceType::King, c, from),
            PieceType::Dragon => self.rook_attacks(from, occupied)   | attacks_from(PieceType::King, c, from),
            _ => Bitboard::ZERO,
        }
    }

    // =========================================================
    //  内部: 長距離利き
    // =========================================================
    pub(crate) fn lance_attacks(&self, c: Color, from: Square, occupied: Bitboard) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        let file = from.file();
        let start_rank = from.rank().to_u8();
        match c {
            Color::Black => {
                for r in (0..start_rank).rev() {
                    let sq = Square::new(file, Rank::from_u8(r).unwrap());
                    bb.set(sq);
                    if occupied.is_set(sq) { break; }
                }
            }
            Color::White => {
                for r in (start_rank + 1)..9 {
                    let sq = Square::new(file, Rank::from_u8(r).unwrap());
                    bb.set(sq);
                    if occupied.is_set(sq) { break; }
                }
            }
        }
        bb
    }

    pub(crate) fn bishop_attacks(&self, from: Square, occupied: Bitboard) -> Bitboard {
        use crate::types::SquareDelta;
        let mut bb = Bitboard::ZERO;
        for &dir in &[SquareDelta::NE, SquareDelta::NW, SquareDelta::SE, SquareDelta::SW] {
            let mut cur = from;
            while let Some(next) = cur + dir {
                bb.set(next);
                if occupied.is_set(next) { break; }
                cur = next;
            }
        }
        bb
    }

    pub(crate) fn rook_attacks(&self, from: Square, occupied: Bitboard) -> Bitboard {
        use crate::types::SquareDelta;
        let mut bb = Bitboard::ZERO;
        for &dir in &[SquareDelta::N, SquareDelta::S, SquareDelta::E, SquareDelta::W] {
            let mut cur = from;
            while let Some(next) = cur + dir {
                bb.set(next);
                if occupied.is_set(next) { break; }
                cur = next;
            }
        }
        bb
    }

    // =========================================================
    //  内部: 打てる場所かチェック
    // =========================================================
    fn can_drop(&self, pt: PieceType, c: Color, to: Square) -> bool {
        let r = to.rank();
        // 行き所のない駒
        match pt {
            PieceType::Pawn | PieceType::Lance => match c {
                Color::Black => if r == Rank::Rank1 { return false; },
                Color::White => if r == Rank::Rank9 { return false; },
            },
            PieceType::Knight => match c {
                Color::Black => if r <= Rank::Rank2 { return false; },
                Color::White => if r >= Rank::Rank8 { return false; },
            },
            _ => {}
        }
        // 二歩
        if pt == PieceType::Pawn {
            let file = to.file();
            let us = c;
            for rank in 0..9 {
                let sq = Square::new(file, Rank::from_u8(rank).unwrap());
                let p = self.piece_at(sq);
                if !p.is_empty() && p.color() == us && p.piece_type() == PieceType::Pawn {
                    return false;
                }
            }
            
            // 打ち歩詰めチェック
            if self.is_uchifuzume(to, c) {
                return false;
            }
        }
        true
    }
    
    /// 打ち歩詰めの判定（やねうら王方式）
    /// 
    /// 歩を打った後、以下をチェック：
    /// 1. 歩で王手になっているか
    /// 2. 歩を取れる駒があるか（pinされていない駒で）
    /// 3. 玉の退路があるか
    /// 
    /// 重要: generate_legal_moves()を呼ばない（無限再帰を避ける）
    fn is_uchifuzume(&self, pawn_sq: Square, c: Color) -> bool {
        // 歩を打った後の局面を作る
        let pawn_drop = Move::new_drop(PieceType::Pawn, pawn_sq);
        let mut test_pos = self.clone();
        test_pos.do_move(pawn_drop);
        
        // 歩で王手になっているか確認
        if !test_pos.in_check() {
            return false;  // 王手じゃないので打ち歩詰めではない
        }
        
        // 敵玉の位置
        let enemy_color = !c;
        let enemy_king_sq = match test_pos.king_square(enemy_color) {
            Some(sq) => sq,
            None => return false,
        };
        
        // --- 1. 歩を取れる駒があるかチェック ---
        // 歩に利いている敵の駒を列挙
        let attackers = test_pos.attackers_to_square(enemy_color, pawn_sq);
        
        if attackers.count() > 0 {
            // pinされていない駒があれば、その駒で取れる
            let pinned = test_pos.pinned_pieces(enemy_color);
            
            // pinされていない駒、または同じ筋の駒（縦方向のpinは取れる）
            let can_capture = attackers & (!pinned | test_pos.file_bb(pawn_sq.file()));
            
            if can_capture.count() > 0 {
                return false;  // 歩を取れるので打ち歩詰めではない
            }
        }
        
        // --- 2. 玉の退路があるかチェック ---
        // 玉が逃げられる升を列挙
        let king_moves = attacks_from(PieceType::King, enemy_color, enemy_king_sq);
        let escape_squares = king_moves & !test_pos.color_bb(enemy_color);  // 自駒がない升
        
        // 歩を置いた升は除外（すでにチェック済み）
        let pawn_mask = Bitboard::square_mask(pawn_sq);
        let escape_squares = escape_squares & !pawn_mask;
        
        // 各退路が安全かチェック
        for escape_sq in escape_squares {
            // 歩を置いた状態での盤面（歩による利きの遮断を考慮）
            let occupied_with_pawn = test_pos.occupied_bb();
            
            // この升に敵の利きがなければ逃げられる
            if !test_pos.is_attacked_by(c, escape_sq, occupied_with_pawn) {
                return false;  // 退路があるので打ち歩詰めではない
            }
        }
        
        // すべてのチェックを抜けたので打ち歩詰め
        true
    }
    
    /// 指定された升に指定された色の駒の利きがあるか
    fn is_attacked_by(&self, attacker_color: Color, target_sq: Square, occupied: Bitboard) -> bool {
        let attacker_pieces = self.color_bb(attacker_color);
        
        for from_sq in attacker_pieces {
            let pt = self.piece_at(from_sq).piece_type();
            let attacks = self.get_attacks(pt, attacker_color, from_sq, occupied);
            
            if attacks.is_set(target_sq) {
                return true;
            }
        }
        
        false
    }
    
    /// 指定された升に利いている指定された色の駒を列挙
    fn attackers_to_square(&self, attacker_color: Color, target_sq: Square) -> Bitboard {
        let mut attackers = Bitboard::ZERO;
        let occupied = self.occupied_bb();
        let attacker_pieces = self.color_bb(attacker_color);
        
        for from_sq in attacker_pieces {
            let pt = self.piece_at(from_sq).piece_type();
            
            // 王・歩・香は打ち歩詰めチェックでは考慮しない
            // （王で取るのは自殺、歩・香で取るのは別の歩・香が必要）
            if pt == PieceType::King || pt == PieceType::Pawn || pt == PieceType::Lance {
                continue;
            }
            
            let attacks = self.get_attacks(pt, attacker_color, from_sq, occupied);
            
            if attacks.is_set(target_sq) {
                attackers.set(from_sq);
            }
        }
        
        attackers
    }
    
    /// pinされている駒を取得
    fn pinned_pieces(&self, c: Color) -> Bitboard {
        let mut pinned = Bitboard::ZERO;
        let king_sq = match self.king_square(c) {
            Some(sq) => sq,
            None => return pinned,
        };
        
        let enemy_color = !c;
        let our_pieces = self.color_bb(c);
        let occupied = self.occupied_bb();
        
        // 飛車・角・龍・馬の遠隔駒でpinをチェック
        let enemy_sliders = self.color_bb(enemy_color) & (
            self.piece_bb(PieceType::Rook) | 
            self.piece_bb(PieceType::Bishop) |
            self.piece_bb(PieceType::Dragon) |
            self.piece_bb(PieceType::Horse)
        );
        
        for slider_sq in enemy_sliders {
            let pt = self.piece_at(slider_sq).piece_type();
            
            // この遠隔駒から玉への直線上に自駒が1つだけあれば、それがpin
            let attacks = self.get_attacks(pt, enemy_color, slider_sq, occupied);
            
            if attacks.is_set(king_sq) {
                // 玉とスライダーの間にある駒を探す
                let between = self.between_bb(slider_sq, king_sq);
                let pieces_between = between & occupied;
                
                // 間に駒が1つだけで、それが自駒ならpin
                if pieces_between.count() == 1 && (pieces_between & our_pieces).count() == 1 {
                    pinned = pinned | pieces_between;
                }
            }
        }
        
        pinned
    }
    
    /// 2つの升の間のBitboardを取得
    fn between_bb(&self, sq1: Square, sq2: Square) -> Bitboard {
        use crate::types::SquareDelta;
        
        let mut bb = Bitboard::ZERO;
        
        // 方向を決定
        let file_diff = sq2.file() as i8 - sq1.file() as i8;
        let rank_diff = sq2.rank().to_u8() as i8 - sq1.rank().to_u8() as i8;
        
        // 斜めまたは縦横でない場合は空
        if file_diff != 0 && rank_diff != 0 && file_diff.abs() != rank_diff.abs() {
            return bb;
        }
        
        let dir = if file_diff == 0 && rank_diff > 0 {
            SquareDelta::N
        } else if file_diff == 0 && rank_diff < 0 {
            SquareDelta::S
        } else if file_diff > 0 && rank_diff == 0 {
            SquareDelta::E
        } else if file_diff < 0 && rank_diff == 0 {
            SquareDelta::W
        } else if file_diff > 0 && rank_diff > 0 {
            SquareDelta::NE
        } else if file_diff < 0 && rank_diff > 0 {
            SquareDelta::NW
        } else if file_diff > 0 && rank_diff < 0 {
            SquareDelta::SE
        } else if file_diff < 0 && rank_diff < 0 {
            SquareDelta::SW
        } else {
            return bb;  // 同じ升
        };
        
        // sq1からsq2に向かって1マスずつ進む
        let mut cur = sq1;
        while let Some(next) = cur + dir {
            if next == sq2 {
                break;
            }
            bb.set(next);
            cur = next;
        }
        
        bb
    }
    
    /// 指定された筋のBitboardを取得
    fn file_bb(&self, file: File) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for rank in 0..9 {
            bb.set(Square::new(file, Rank::from_u8(rank).unwrap()));
        }
        bb
    }
    
    fn must_promote(&self, pt: PieceType, c: Color, to: Square) -> bool {
        let r = to.rank();
        match pt {
            // 行き所のない駒（元々の実装）
            PieceType::Pawn | PieceType::Lance => match c {
                Color::Black => r == Rank::Rank1,
                Color::White => r == Rank::Rank9,
            },
            PieceType::Knight => match c {
                Color::Black => r <= Rank::Rank2,
                Color::White => r >= Rank::Rank8,
            },
            // 飛車・角は敵陣（1-3段目または7-9段目）で強制成り
            PieceType::Rook | PieceType::Bishop => {
                match c {
                    Color::Black => r.to_u8() <= 2,  // 1-3段目
                    Color::White => r.to_u8() >= 6,  // 7-9段目
                }
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startpos_legal_moves() {
        let pos = Position::start_position();
        let moves = pos.generate_legal_moves();
        // 初期局面: 30手
        assert_eq!(moves.len(), 30, "Expected 30 moves, got {}", moves.len());
    }

    #[test]
    fn test_nifu_prevented() {
        // 5筋に先手の歩がいる局面
        let sfen = "4k4/9/9/9/4P4/9/9/9/4K4 b P 1";
        let pos = Position::from_sfen(sfen).unwrap();
        let moves = pos.generate_legal_moves();
        // 5筋への歩打ちは二歩なので含まれてはいけない
        let illegal = moves.iter().any(|m| {
            m.is_drop()
                && m.piece_type_dropped() == PieceType::Pawn
                && m.to().file() == File::File5
        });
        assert!(!illegal, "Nifu (double pawn) should be prevented");
    }

    #[test]
    fn test_check_evasion() {
        // 後手の飛車が先手玉に王手している局面（先手番）
        // 5i に先手玉、5a に後手飛
        let sfen = "4r4/9/9/9/9/9/9/9/4K4 b - 1";
        let pos = Position::from_sfen(sfen).unwrap();
        // 王手がかかっているので合法手は全て回避手のはず
        let moves = pos.generate_legal_moves();
        assert!(moves.len() > 0, "Should have evasion moves");
        // 全ての合法手は王手放置していないこと（legal()が保証）
        for mv in &moves {
            let mut after = pos.clone();
            after.do_move(*mv);
            assert!(!after.is_attacked(Color::Black), 
                    "Move {} leaves king in check", mv.to_usi());
        }
    }

    #[test]
    fn test_no_move_through_own_pieces() {
        let pos = Position::start_position();
        let moves = pos.generate_legal_moves();
        // 初期局面では角・飛は動けない
        let bishop_rook_moves: Vec<_> = moves.iter().filter(|m| {
            if m.is_drop() { return false; }
            let pt = pos.piece_at(m.from().unwrap()).piece_type();
            pt == PieceType::Bishop || pt == PieceType::Rook
        }).collect();
        assert!(bishop_rook_moves.is_empty(), 
                "Bishop/Rook should not move in start position");
    }
}
