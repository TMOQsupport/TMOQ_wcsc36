// mate1ply.rs - Phase 3（TMOQ互換版）
// TMOQの既存メソッドに合わせて修正

use crate::bitboard::Bitboard;
use crate::types::{Color, Square};
use crate::piece::{PieceType, HandPiece};
use crate::position::Position;
use crate::r#move::Move;
use crate::attacks::attacks_from;

// ============================================================
//  駒の利き計算
// ============================================================

pub fn rook_step_effect(sq: Square) -> Bitboard {
    attacks_from(PieceType::Rook, Color::Black, sq)
}

pub fn bishop_step_effect(sq: Square) -> Bitboard {
    attacks_from(PieceType::Silver, Color::Black, sq)
}

pub fn gold_effect(c: Color, sq: Square) -> Bitboard {
    attacks_from(PieceType::Gold, c, sq)
}

pub fn silver_effect(c: Color, sq: Square) -> Bitboard {
    attacks_from(PieceType::Silver, c, sq)
}

pub fn knight_effect(c: Color, sq: Square) -> Bitboard {
    attacks_from(PieceType::Knight, c, sq)
}

pub fn pawn_effect(c: Color, sq: Square) -> Bitboard {
    attacks_from(PieceType::Pawn, c, sq)
}

pub fn king_effect(sq: Square) -> Bitboard {
    attacks_from(PieceType::King, Color::Black, sq)
}

// ============================================================
//  Positionへの拡張メソッド
// ============================================================

impl Position {
    /// 指定マスに指定色の駒の利きがあるか（簡易実装）
    pub fn has_attacker_to(&self, us: Color, sq: Square) -> bool {
        // 全ての駒種について、sqに利きがあるかチェック
        let piece_types = [
            PieceType::Pawn, PieceType::Lance, PieceType::Knight,
            PieceType::Silver, PieceType::Gold, PieceType::Bishop,
            PieceType::Rook, PieceType::King,
            PieceType::ProPawn, PieceType::ProLance, PieceType::ProKnight,
            PieceType::ProSilver, PieceType::Horse, PieceType::Dragon,
        ];
        
        for &pt in &piece_types {
            let attacks = attacks_from(pt, !us, sq);
            let our_pieces = self.pieces_of(us, pt);
            
            if (attacks & our_pieces).is_any() {
                return true;
            }
        }
        
        false
    }
    
    /// 1手詰め判定（Phase 3）
    pub fn mate1ply_fast(&self) -> Option<Move> {
        // 既に王手がかかっている場合はスキップ
        if self.in_check() {
            return None;
        }
        
        let us = self.side_to_move();
        let them = !us;
        let sq_king = self.king_square(them)?;
        
        let bb_drop = !self.occupied_bb();
        let occupied = self.occupied_bb();
        let our_hand = self.hand(us);
        let pinned = Bitboard::ZERO;
        
        // 飛車打ち
        if our_hand.count(HandPiece::HRook) > 0 {
            let bb = rook_step_effect(sq_king) & king_effect(sq_king) & bb_drop;
            
            for sq in bb.iter() {
                if !self.has_attacker_to(us, sq) {
                    continue;
                }
                
                let drop_attacks = rook_step_effect(sq);
                if can_king_escape(self, them, sq, drop_attacks, occupied) {
                    continue;
                }
                if can_piece_capture(self, them, sq, pinned, occupied) {
                    continue;
                }
                
                return Some(Move::new_drop(PieceType::Rook, sq));
            }
        }
        
        // 角打ち
        if our_hand.count(HandPiece::HBishop) > 0 {
            let bb = bishop_step_effect(sq_king) & king_effect(sq_king) & bb_drop;
            
            for sq in bb.iter() {
                if !self.has_attacker_to(us, sq) {
                    continue;
                }
                
                let drop_attacks = bishop_step_effect(sq);
                if can_king_escape(self, them, sq, drop_attacks, occupied) {
                    continue;
                }
                if can_piece_capture(self, them, sq, pinned, occupied) {
                    continue;
                }
                
                return Some(Move::new_drop(PieceType::Bishop, sq));
            }
        }
        
        // 金打ち
        if our_hand.count(HandPiece::HGold) > 0 {
            let mut bb = gold_effect(them, sq_king) & bb_drop;
            
            // 強い駒で詰む場所は除外
            if our_hand.count(HandPiece::HRook) > 0 {
                bb &= !rook_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HBishop) > 0 {
                bb &= !bishop_step_effect(sq_king);
            }
            
            for sq in bb.iter() {
                if !self.has_attacker_to(us, sq) {
                    continue;
                }
                
                let drop_attacks = gold_effect(us, sq);
                if can_king_escape(self, them, sq, drop_attacks, occupied) {
                    continue;
                }
                if can_piece_capture(self, them, sq, pinned, occupied) {
                    continue;
                }
                
                return Some(Move::new_drop(PieceType::Gold, sq));
            }
        }
        
        // 銀打ち
        if our_hand.count(HandPiece::HSilver) > 0 {
            let mut bb = silver_effect(them, sq_king) & bb_drop;
            
            if our_hand.count(HandPiece::HRook) > 0 {
                bb &= !rook_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HBishop) > 0 {
                bb &= !bishop_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HGold) > 0 {
                bb &= !gold_effect(them, sq_king);
            }
            
            for sq in bb.iter() {
                if !self.has_attacker_to(us, sq) {
                    continue;
                }
                
                let drop_attacks = silver_effect(us, sq);
                if can_king_escape(self, them, sq, drop_attacks, occupied) {
                    continue;
                }
                if can_piece_capture(self, them, sq, pinned, occupied) {
                    continue;
                }
                
                return Some(Move::new_drop(PieceType::Silver, sq));
            }
        }
        
        // 桂打ち
        if our_hand.count(HandPiece::HKnight) > 0 {
            let mut bb = knight_effect(them, sq_king) & bb_drop;
            
            let rank = sq_king.rank().to_u8();
            if us == Color::Black && rank <= 1 {
                bb = Bitboard::ZERO;
            } else if us == Color::White && rank >= 7 {
                bb = Bitboard::ZERO;
            }
            
            if our_hand.count(HandPiece::HRook) > 0 {
                bb &= !rook_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HBishop) > 0 {
                bb &= !bishop_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HGold) > 0 {
                bb &= !gold_effect(them, sq_king);
            }
            if our_hand.count(HandPiece::HSilver) > 0 {
                bb &= !silver_effect(them, sq_king);
            }
            
            for sq in bb.iter() {
                if !self.has_attacker_to(us, sq) {
                    continue;
                }
                
                let drop_attacks = knight_effect(us, sq);
                if can_king_escape(self, them, sq, drop_attacks, occupied) {
                    continue;
                }
                if can_piece_capture(self, them, sq, pinned, occupied) {
                    continue;
                }
                
                return Some(Move::new_drop(PieceType::Knight, sq));
            }
        }
        
        // 歩打ち
        if our_hand.count(HandPiece::HPawn) > 0 {
            let mut bb = pawn_effect(them, sq_king) & bb_drop;
            
            let rank = sq_king.rank().to_u8();
            if us == Color::Black && rank == 0 {
                bb = Bitboard::ZERO;
            } else if us == Color::White && rank == 8 {
                bb = Bitboard::ZERO;
            }
            
            if our_hand.count(HandPiece::HRook) > 0 {
                bb &= !rook_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HBishop) > 0 {
                bb &= !bishop_step_effect(sq_king);
            }
            if our_hand.count(HandPiece::HGold) > 0 {
                bb &= !gold_effect(them, sq_king);
            }
            if our_hand.count(HandPiece::HSilver) > 0 {
                bb &= !silver_effect(them, sq_king);
            }
            if our_hand.count(HandPiece::HKnight) > 0 {
                bb &= !knight_effect(them, sq_king);
            }
            
            for sq in bb.iter() {
                // 二歩チェック
                if self.has_pawn_on_file(us, sq.file()) {
                    continue;
                }
                
                if !self.has_attacker_to(us, sq) {
                    continue;
                }
                
                let drop_attacks = pawn_effect(us, sq);
                if can_king_escape(self, them, sq, drop_attacks, occupied) {
                    continue;
                }
                if can_piece_capture(self, them, sq, pinned, occupied) {
                    continue;
                }
                
                return Some(Move::new_drop(PieceType::Pawn, sq));
            }
        }
        
        None
    }
    
    /// 二歩チェック
    pub fn has_pawn_on_file(&self, us: Color, file: crate::types::File) -> bool {
        use crate::types::Rank;
        
        for rank_u8 in 0..=8 {
            if let Some(rank) = Rank::from_u8(rank_u8) {
                let sq = Square::new(file, rank);
                let piece = self.piece_at(sq);
                
                if !piece.is_empty() && piece.color() == us && piece.piece_type() == PieceType::Pawn {
                    return true;
                }
            }
        }
        
        false
    }
}

// ============================================================
//  詰み判定の補助関数
// ============================================================

fn can_king_escape(
    pos: &Position,
    them: Color,
    drop_to: Square,
    drop_attacks: Bitboard,
    _occupied: Bitboard,
) -> bool {
    let sq_king = match pos.king_square(them) {
        Some(sq) => sq,
        None => return true,
    };
    
    let king_moves = king_effect(sq_king);
    let us = !them;
    let our_pieces = pos.pieces_of_color_bb(us);
    let mut escapes = king_moves & !our_pieces;
    
    escapes &= !drop_attacks;
    escapes &= !Bitboard::square_mask(drop_to);
    
    for escape_sq in escapes.iter() {
        if !pos.has_attacker_to(us, escape_sq) {
            return true;
        }
    }
    
    false
}

fn can_piece_capture(
    pos: &Position,
    them: Color,
    drop_to: Square,
    _pinned: Bitboard,
    _occupied: Bitboard,
) -> bool {
    pos.has_attacker_to(them, drop_to)
}
