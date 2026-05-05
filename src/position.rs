// position.rs - 盤面状態の管理（cshogiのposition.hpp/cppより）
// use crate::eval_params::get_eval_params;
use crate::eval_params::{get_eval_params, EvalParams};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{Color, Square, File, Rank};
use crate::piece::{Piece, PieceType, HandPiece};
use crate::bitboard::Bitboard;
use crate::hand::Hand;
use crate::r#move::Move;

// グローバルな乱数生成器（簡易版）
static RNG_STATE: AtomicU64 = AtomicU64::new(12345);

fn simple_rand() -> u32 {
    // 単純なLCG (Linear Congruential Generator)
    let state = RNG_STATE.load(Ordering::Relaxed);
    let new_state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    RNG_STATE.store(new_state, Ordering::Relaxed);
    (new_state >> 32) as u32
}


/// 盤面状態
#[derive(Clone)]
pub struct Position {
    /// 盤上の駒 [Square]
    board: [Piece; 81],
    
    /// 各駒種のBitboard [PieceType]
    piece_bb: [Bitboard; 15],
    
    /// 各色のBitboard [Color]
    color_bb: [Bitboard; 2],
    
    /// 持ち駒 [Color]
    hand: [Hand; 2],
    
    /// 手番
    side_to_move: Color,
    
    /// 手数
    ply: u32,
}

impl Position {
    /// 空の盤面を作成
    pub fn empty() -> Position {
        Position {
            board: [Piece::EMPTY; 81],
            piece_bb: [Bitboard::ZERO; 15],
            color_bb: [Bitboard::ZERO; 2],
            hand: [Hand::EMPTY; 2],
            side_to_move: Color::Black,
            ply: 0,
        }
    }
    
    /// 初期局面を作成
    pub fn start_position() -> Position {
        Position::from_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap()
    }
    
    /// 指定位置の駒を取得
    #[inline]
    pub fn piece_at(&self, sq: Square) -> Piece {
        self.board[sq.0 as usize]
    }
    
    /// 手番を取得
    #[inline]
    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }
    
    /// 持ち駒を取得
    #[inline]
    pub fn hand(&self, c: Color) -> Hand {
        self.hand[c as usize]
    }
    
    /// 指定色の駒のBitboardを取得
    #[inline]
    pub fn color_bb(&self, c: Color) -> Bitboard {
        self.color_bb[c as usize]
    }
    
    /// 指定駒種のBitboardを取得
    #[inline]
    pub fn piece_bb(&self, pt: PieceType) -> Bitboard {
        self.piece_bb[pt as usize]
    }
    
    /// 全駒のBitboardを取得
    #[inline]
    pub fn occupied_bb(&self) -> Bitboard {
        self.color_bb[Color::Black as usize] | self.color_bb[Color::White as usize]
    }
    
    /// 玉の位置を取得
    pub fn king_square(&self, c: Color) -> Option<Square> {
        let king_bb = self.piece_bb(PieceType::King) & self.color_bb(c);
        king_bb.first()
    }
    
    /// 香の利き（公開ラッパー）
    pub fn lance_attacks_pub(&self, c: Color, from: Square, occupied: Bitboard) -> Bitboard {
        self.lance_attacks(c, from, occupied)
    }
    
    /// 角の利き（公開ラッパー）
    pub fn bishop_attacks_pub(&self, from: Square, occupied: Bitboard) -> Bitboard {
        self.bishop_attacks(from, occupied)
    }
    
    /// 飛の利き（公開ラッパー）
    pub fn rook_attacks_pub(&self, from: Square, occupied: Bitboard) -> Bitboard {
        self.rook_attacks(from, occupied)
    }
    
    /// 駒を配置
    fn put_piece(&mut self, piece: Piece, sq: Square) {
        self.board[sq.0 as usize] = piece;
        let pt = piece.piece_type();
        let c = piece.color();
        self.piece_bb[pt as usize].set(sq);
        self.color_bb[c as usize].set(sq);
    }
    
    /// 駒を除去
    fn remove_piece(&mut self, sq: Square) -> Piece {
        let piece = self.board[sq.0 as usize];
        if !piece.is_empty() {
            self.board[sq.0 as usize] = Piece::EMPTY;
            let pt = piece.piece_type();
            let c = piece.color();
            self.piece_bb[pt as usize].clear(sq);
            self.color_bb[c as usize].clear(sq);
        }
        piece
    }
    
    /// 指し手を実行
    pub fn do_move(&mut self, mv: Move) {
        if mv.is_drop() {
            // 駒打ち
            let pt = mv.piece_type_dropped();
            let to = mv.to();
            let piece = Piece::new(self.side_to_move, pt);
            
            self.put_piece(piece, to);
            self.hand[self.side_to_move as usize].remove_piece(pt);
        } else {
            // 通常の移動
            let from = mv.from().unwrap();
            let to = mv.to();
            
            // 移動元の駒を取得
            let mut piece = self.remove_piece(from);
            
            // 移動先の駒を取得（取る場合）
            let captured = self.remove_piece(to);
            if !captured.is_empty() {
                // 取った駒を持ち駒に追加（成りを戻す）
                let captured_pt = captured.piece_type().unpromote();
                self.hand[self.side_to_move as usize].add_piece(captured_pt);
            }
            
            // 成る場合
            if mv.is_promotion() {
                piece = piece.promote().unwrap();
            }
            
            // 移動先に配置
            self.put_piece(piece, to);
        }
        
        // 手番交代
        self.side_to_move = self.side_to_move.opposite();
        self.ply += 1;
    }
    
    /// 簡易的な評価値を計算
    /// Null move（手番だけ変える）を適用した新しい局面を返す
    /// 注意: 手数は進めない（やねうら王の do_null_move に準拠）
    pub fn make_null_move(&self) -> Position {
        let mut pos = self.clone();
        pos.side_to_move = pos.side_to_move.opposite();
        // 手数は進めない（null moveは実際の指し手ではない）
        pos
    }

    pub fn evaluate(&self) -> i32 {
        let mut score = 0;
        
        // 盤上の駒
        for sq_idx in 0..81 {
            let piece = self.board[sq_idx];
            if !piece.is_empty() {
                let pt = piece.piece_type();
                let value = crate::piece::piece_value(pt);
                
                if piece.color() == Color::Black {
                    score += value;
                } else {
                    score -= value;
                }
            }
        }
        
        // 持ち駒
        for hp_idx in 0..7 {
            let hp = unsafe { std::mem::transmute::<u8, HandPiece>(hp_idx) };
            let pt = hp.to_piece_type();
            let value = crate::piece::piece_value(pt);
            
            let black_count = self.hand[Color::Black as usize].count(hp) as i32;
            let white_count = self.hand[Color::White as usize].count(hp) as i32;
            
            score += value * black_count;
            score -= value * white_count;
        }
        
        score
    }
    
    /// SFEN形式から局面を作成
    pub fn from_sfen(sfen: &str) -> Option<Position> {
        let parts: Vec<&str> = sfen.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }
        
        let mut pos = Position::empty();
        
        // 1. 盤面（SFENは9筋から1筋、1段から9段の順）
        let board_str = parts[0];
        let mut file = 8;  // 9筋から開始（内部は0-8）
        let mut rank = 0;  // 1段から開始
        let mut promoted = false;
        
        for ch in board_str.chars() {
            if ch == '/' {
                // 次の段へ
                rank += 1;
                file = 8;  // 9筋に戻る
                continue;
            } else if ch == '+' {
                promoted = true;
                continue;
            } else if ch.is_ascii_digit() {
                // 空マスの数
                let empty = ch.to_digit(10)? as i8;
                file -= empty;
                continue;
            }
            
            // 駒
            let mut piece = Piece::from_sfen_char(ch)?;
            if promoted {
                piece = piece.promote()?;
                promoted = false;
            }
            
            if file < 0 || rank >= 9 {
                return None;
            }
            
            let sq = Square::new(
                File::from_u8(file as u8)?,
                Rank::from_u8(rank)?
            );
            pos.put_piece(piece, sq);
            file -= 1;
        }
        
        // 2. 手番
        pos.side_to_move = match parts[1] {
            "b" => Color::Black,
            "w" => Color::White,
            _ => return None,
        };
        
        // 3. 持ち駒
        if parts[2] != "-" {
            let hand_str = parts[2];
            let mut chars = hand_str.chars().peekable();
            
            while let Some(ch) = chars.next() {
                // 数字の場合は枚数
                let count = if ch.is_ascii_digit() {
                    let mut num_str = String::new();
                    num_str.push(ch);
                    
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() {
                            num_str.push(next_ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    num_str.parse::<u32>().ok()?
                } else {
                    1
                };
                
                // 駒の種類
                let piece_char = if ch.is_ascii_digit() {
                    chars.next()?
                } else {
                    ch
                };
                
                let color = if piece_char.is_uppercase() {
                    Color::Black
                } else {
                    Color::White
                };
                
                let pt = match piece_char.to_uppercase().next()? {
                    'P' => PieceType::Pawn,
                    'L' => PieceType::Lance,
                    'N' => PieceType::Knight,
                    'S' => PieceType::Silver,
                    'G' => PieceType::Gold,
                    'B' => PieceType::Bishop,
                    'R' => PieceType::Rook,
                    _ => return None,
                };
                
                let hp = HandPiece::from_piece_type(pt)?;
                pos.hand[color as usize].set(hp, count);
            }
        }
        
        // 4. 手数（オプション）
        if parts.len() >= 4 {
            pos.ply = parts[3].parse::<u32>().unwrap_or(1);
        }
        
        Some(pos)
    }
    
    /// SFEN形式に変換
    pub fn to_sfen(&self) -> String {
        let mut sfen = String::new();
        
        // 1. 盤面
        for rank in 0..9 {
            let mut empty_count = 0;
            
            for file in (0..9).rev() {
                let sq = Square::new(
                    File::from_u8(file).unwrap(),
                    Rank::from_u8(rank).unwrap()
                );
                let piece = self.piece_at(sq);
                
                if piece.is_empty() {
                    empty_count += 1;
                } else {
                    if empty_count > 0 {
                        sfen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    sfen.push_str(&piece.to_sfen());
                }
            }
            
            if empty_count > 0 {
                sfen.push_str(&empty_count.to_string());
            }
            
            if rank < 8 {
                sfen.push('/');
            }
        }
        
        // 2. 手番
        sfen.push(' ');
        sfen.push(match self.side_to_move {
            Color::Black => 'b',
            Color::White => 'w',
        });
        
        // 3. 持ち駒
        sfen.push(' ');
        let black_hand = self.hand[Color::Black as usize].to_sfen();
        let white_hand = self.hand[Color::White as usize].to_sfen();
        
        if black_hand == "-" && white_hand == "-" {
            sfen.push('-');
        } else {
            if black_hand != "-" {
                sfen.push_str(&black_hand);
            }
            if white_hand != "-" {
                sfen.push_str(&white_hand.to_lowercase());
            }
        }
        
        // 4. 手数
        sfen.push(' ');
        sfen.push_str(&self.ply.to_string());
        
        sfen
    }
    /// 全駒のBitboardを取得
    #[inline]
    pub fn pieces(&self) -> Bitboard {
        self.occupied_bb()
    }
    pub fn pieces_of(&self, color: Color, pt: PieceType) -> Bitboard {
        self.piece_bb[pt as usize] & self.color_bb[color as usize]
    }
    pub fn pieces_of_color_bb(&self, color: Color) -> Bitboard {
        self.color_bb[color as usize]
    }
    pub fn can_drop_pawn(&self, file: u8) -> bool {
        // 自分の歩がその筋にないかチェック
        let us = self.side_to_move;
        for rank in 0..9 {
            if let (Some(f), Some(r)) = (File::from_u8(file), Rank::from_u8(rank)) {
                let sq = Square::new(f, r);
                let piece = self.piece_at(sq);
                if !piece.is_empty() && piece.color() == us && piece.piece_type() == PieceType::Pawn {
                    return false;
                }
            }
        }
        true
    }
    
    pub fn mate1ply(&self) -> Option<Move> {
        self.mate1ply_fast()
    }
    
	pub fn evaluate_improved(&self) -> i32 {
		let params = get_eval_params();
		let mut score = 0;
		
		// 1. 駒得評価（持ち駒も含む）
		score += self.evaluate_material_with_hand(params);
		
		// 2. 位置評価
		score += self.evaluate_simple_position(params);
		
		// 3. 玉の安全度
		score += self.evaluate_king_safety(params);
		
		// 4. 手番の価値
		score += self.evaluate_tempo(params);
		
		// 5. ランダムノイズ（params.jsonで設定可能）
		if params.eval_noise > 0 {
			let range = params.eval_noise * 2 + 1;
			let noise = (simple_rand() % range as u32) as i32 - params.eval_noise;
			score += noise;
		}
		
		// 5. 小さなランダムノイズを追加（±3点）
        // これにより、同じ局面でも少し異なる評価になる
		//let mut noise_param1 = 101;
		//let mut noise_param2 = 50;
        //let noise = (simple_rand() % noise_param2) as i32 - noise_param1;  // -3 ~ +3
        //score += noise;
		
		score
	}

	/// 駒得評価（持ち駒込み）- パラメータ化版
	fn evaluate_material_with_hand(&self, params: &EvalParams) -> i32 {
		use crate::piece::piece_value;
		
		let mut score = 0;
		
		// 盤上の駒
		for i in 0..81 {
			if let Some(sq) = Square::from_u8(i) {
				let piece = self.piece_at(sq);
				if piece.is_empty() {
					continue;
				}
				
				let value = piece_value(piece.piece_type());
				
				if piece.color() == Color::Black {
					score += value;
				} else {
					score -= value;
				}
			}
		}
		
		// 持ち駒（パラメータから取得）
		for color in [Color::Black, Color::White] {
			let hand = self.hand(color);
			let hand_value = 
				hand.count(HandPiece::HPawn) as i32 * params.hand_pawn +
				hand.count(HandPiece::HLance) as i32 * params.hand_lance +
				hand.count(HandPiece::HKnight) as i32 * params.hand_knight +
				hand.count(HandPiece::HSilver) as i32 * params.hand_silver +
				hand.count(HandPiece::HGold) as i32 * params.hand_gold +
				hand.count(HandPiece::HBishop) as i32 * params.hand_bishop +
				hand.count(HandPiece::HRook) as i32 * params.hand_rook;
			
			if color == Color::Black {
				score += hand_value;
			} else {
				score -= hand_value;
			}
		}
		
		score
	}

	/// 簡易位置評価 - パラメータ化版
	fn evaluate_simple_position(&self, params: &EvalParams) -> i32 {
		let mut score = 0;
		
		let black_king = self.king_square(Color::Black);
		let white_king = self.king_square(Color::White);
		
		for i in 0..81 {
			if let Some(sq) = Square::from_u8(i) {
				let piece = self.piece_at(sq);
				if piece.is_empty() {
					continue;
				}
				
				let pt = piece.piece_type();
				let color = piece.color();
				
				if pt == PieceType::King {
					continue;
				}
				
				let mut pos_bonus = 0;
				
				// 敵玉への近接ボーナス（パラメータから重みを取得）
				let enemy_king = if color == Color::Black {
					white_king
				} else {
					black_king
				};
				
				if let Some(king_sq) = enemy_king {
					let dist = self.manhattan_distance(sq, king_sq);
					
					match pt {
						PieceType::Rook | PieceType::Dragon => {
							pos_bonus += (14 - dist.min(14)) * params.rook_proximity_weight;
						}
						PieceType::Bishop | PieceType::Horse => {
							pos_bonus += (14 - dist.min(14)) * params.bishop_proximity_weight;
						}
						PieceType::Gold | PieceType::ProPawn | 
						PieceType::ProLance | PieceType::ProKnight | 
						PieceType::ProSilver => {
							pos_bonus += (14 - dist.min(14)) * params.gold_proximity_weight;
						}
						_ => {}
					}
				}
				
				// 敵陣にいる駒にボーナス（パラメータから値を取得）
				let in_enemy_zone = if color == Color::Black {
					sq.rank().to_u8() <= 2
				} else {
					sq.rank().to_u8() >= 6
				};
				
				if in_enemy_zone {
					match pt {
						PieceType::Gold | PieceType::ProPawn | 
						PieceType::ProLance | PieceType::ProKnight | 
						PieceType::ProSilver => {
							pos_bonus += params.enemy_zone_gold;
						}
						PieceType::Horse => {
							pos_bonus += params.enemy_zone_horse;
						}
						PieceType::Dragon => {
							pos_bonus += params.enemy_zone_dragon;
						}
						PieceType::Rook => {
							pos_bonus += params.enemy_zone_rook;
						}
						PieceType::Bishop => {
							pos_bonus += params.enemy_zone_bishop;
						}
						PieceType::Pawn | PieceType::Lance | 
						PieceType::Knight | PieceType::Silver => {
							pos_bonus += params.enemy_zone_pawn;
						}
						_ => {}
					}
				}
				
				// 中央制御ボーナス（パラメータから値を取得）
				let file = sq.file().to_u8();
				if file >= 3 && file <= 5 {
					match pt {
						PieceType::Rook | PieceType::Dragon => {
							pos_bonus += params.center_rook;
						}
						PieceType::Bishop | PieceType::Horse => {
							pos_bonus += params.center_bishop;
						}
						PieceType::Silver | PieceType::Gold => {
							pos_bonus += params.center_gold;
						}
						_ => {}
					}
				}
				
				if color == Color::Black {
					score += pos_bonus;
				} else {
					score -= pos_bonus;
				}
			}
		}
		
		score
	}

	/// 玉の安全度評価 - パラメータ化版
	fn evaluate_king_safety(&self, params: &EvalParams) -> i32 {
		let mut score = 0;
		
		for color in [Color::Black, Color::White] {
			if let Some(king_sq) = self.king_square(color) {
				let mut safety = 0;
				
				// 玉の周囲8マスをチェック（既存のget_king_neighbors関数を使用）
				let king_neighbors = self.get_king_neighbors(king_sq);
				
				for neighbor_sq in king_neighbors {
					let piece = self.piece_at(neighbor_sq);
					
					if !piece.is_empty() && piece.color() == color {
						let pt = piece.piece_type();
						
						match pt {
							PieceType::Gold => safety += params.king_safety_gold,
							PieceType::Silver => safety += params.king_safety_silver,
							PieceType::ProPawn | PieceType::ProLance | 
							PieceType::ProKnight | PieceType::ProSilver => {
								safety += params.king_safety_propawn;
							}
							PieceType::Pawn => safety += params.king_safety_pawn,
							_ => {}
						}
					}
				}
				
				// 玉の位置ペナルティ
				let file = king_sq.file().to_u8();
				
				// 端にいると安全
				if file == 0 || file == 8 {
					safety += params.king_edge_bonus;
				} else if file >= 3 && file <= 5 {
					// 中央は危険
					safety += params.king_center_penalty;
				}
				
				if color == Color::Black {
					score += safety;
				} else {
					score -= safety;
				}
			}
		}
		
		score
	}

	/// 手番の価値 - パラメータ化版
	fn evaluate_tempo(&self, params: &EvalParams) -> i32 {
		if self.side_to_move() == Color::Black {
			params.tempo_value
		} else {
			-params.tempo_value
		}
	}

    
    /// マンハッタン距離
    fn manhattan_distance(&self, sq1: Square, sq2: Square) -> i32 {
        let file_diff = (sq1.file().to_u8() as i32 - sq2.file().to_u8() as i32).abs();
        let rank_diff = (sq1.rank().to_u8() as i32 - sq2.rank().to_u8() as i32).abs();
        file_diff + rank_diff
    }
    
   
    /// 玉の8近傍を取得
    fn get_king_neighbors(&self, king_sq: Square) -> Vec<Square> {
        let file = king_sq.file().to_u8() as i32;
        let rank = king_sq.rank().to_u8() as i32;
        
        let mut neighbors = Vec::new();
        
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                
                let new_file = file + df;
                let new_rank = rank + dr;
                
                if new_file >= 0 && new_file < 9 && new_rank >= 0 && new_rank < 9 {
                    if let (Some(f), Some(r)) = (
                        File::from_u8(new_file as u8),
                        Rank::from_u8(new_rank as u8)
                    ) {
                        neighbors.push(Square::new(f, r));
                    }
                }
            }
        }
        
        neighbors
    }
}

