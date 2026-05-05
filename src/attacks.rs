// attacks.rs - 駒の利き計算（cshogiのinit.cpp, bitboard.cppより）

use crate::types::{Square, Rank, Color, SquareDelta};
use crate::bitboard::Bitboard;
use crate::piece::PieceType;

/// 攻撃範囲テーブル
pub struct AttackTables {
    // 歩の利き [Color][Square]
    pawn: [[Bitboard; 81]; 2],
    // 桂の利き [Color][Square]
    knight: [[Bitboard; 81]; 2],
    // 銀の利き [Color][Square]
    silver: [[Bitboard; 81]; 2],
    // 金の利き [Color][Square]
    gold: [[Bitboard; 81]; 2],
    // 玉の利き [Square]
    king: [Bitboard; 81],
    // 香の利き [Color][Square][Occupied pattern] - 簡易版
    lance: Box<[[[Bitboard; 128]; 81]; 2]>,
}

impl AttackTables {
    /// 攻撃範囲テーブルを初期化
    pub fn new() -> Self {
        let mut tables = AttackTables {
            pawn: [[Bitboard::ZERO; 81]; 2],
            knight: [[Bitboard::ZERO; 81]; 2],
            silver: [[Bitboard::ZERO; 81]; 2],
            gold: [[Bitboard::ZERO; 81]; 2],
            king: [Bitboard::ZERO; 81],
            lance: Box::new([[[Bitboard::ZERO; 128]; 81]; 2]),
        };
        
        tables.init_step_attacks();
        tables.init_lance_attacks();
        tables
    }
    
    /// 歩、桂、銀、金、玉の利きを初期化
    fn init_step_attacks(&mut self) {
        for sq_idx in 0..81 {
            let sq = Square(sq_idx);
            
            // 先手の歩: 1マス前
            if let Some(to) = sq + SquareDelta::N {
                self.pawn[Color::Black as usize][sq_idx as usize].set(to);
            }
            
            // 後手の歩: 1マス後
            if let Some(to) = sq + SquareDelta::S {
                self.pawn[Color::White as usize][sq_idx as usize].set(to);
            }
            
            // 先手の桂: 前2マス、左右1マス
            if let Some(to) = sq + SquareDelta::N {
                if let Some(to) = to + SquareDelta::N {
                    if let Some(to_left) = to + SquareDelta::W {
                        self.knight[Color::Black as usize][sq_idx as usize].set(to_left);
                    }
                    if let Some(to_right) = to + SquareDelta::E {
                        self.knight[Color::Black as usize][sq_idx as usize].set(to_right);
                    }
                }
            }
            
            // 後手の桂: 後2マス、左右1マス
            if let Some(to) = sq + SquareDelta::S {
                if let Some(to) = to + SquareDelta::S {
                    if let Some(to_left) = to + SquareDelta::W {
                        self.knight[Color::White as usize][sq_idx as usize].set(to_left);
                    }
                    if let Some(to_right) = to + SquareDelta::E {
                        self.knight[Color::White as usize][sq_idx as usize].set(to_right);
                    }
                }
            }
            
            // 銀の利き（先手: 前3方向、斜め後2方向）
            let silver_dirs_black = [
                SquareDelta::N,   // 前
                SquareDelta::NE,  // 右斜め前
                SquareDelta::NW,  // 左斜め前
                SquareDelta::SE,  // 右斜め後
                SquareDelta::SW,  // 左斜め後
            ];
            for &dir in &silver_dirs_black {
                if let Some(to) = sq + dir {
                    self.silver[Color::Black as usize][sq_idx as usize].set(to);
                }
            }
            
            // 銀の利き（後手）
            let silver_dirs_white = [
                SquareDelta::S,
                SquareDelta::SE,
                SquareDelta::SW,
                SquareDelta::NE,
                SquareDelta::NW,
            ];
            for &dir in &silver_dirs_white {
                if let Some(to) = sq + dir {
                    self.silver[Color::White as usize][sq_idx as usize].set(to);
                }
            }
            
            // 金の利き（先手: 前3方向、横2方向、後1方向）
            let gold_dirs_black = [
                SquareDelta::N,   // 前
                SquareDelta::NE,  // 右斜め前
                SquareDelta::NW,  // 左斜め前
                SquareDelta::E,   // 右
                SquareDelta::W,   // 左
                SquareDelta::S,   // 後
            ];
            for &dir in &gold_dirs_black {
                if let Some(to) = sq + dir {
                    self.gold[Color::Black as usize][sq_idx as usize].set(to);
                }
            }
            
            // 金の利き（後手）
            let gold_dirs_white = [
                SquareDelta::S,
                SquareDelta::SE,
                SquareDelta::SW,
                SquareDelta::E,
                SquareDelta::W,
                SquareDelta::N,
            ];
            for &dir in &gold_dirs_white {
                if let Some(to) = sq + dir {
                    self.gold[Color::White as usize][sq_idx as usize].set(to);
                }
            }
            
            // 玉の利き（全方向1マス）
            let king_dirs = [
                SquareDelta::N, SquareDelta::S, SquareDelta::E, SquareDelta::W,
                SquareDelta::NE, SquareDelta::NW, SquareDelta::SE, SquareDelta::SW,
            ];
            for &dir in &king_dirs {
                if let Some(to) = sq + dir {
                    self.king[sq_idx as usize].set(to);
                }
            }
        }
    }
    
    /// 香の利きを初期化（簡易版）
    fn init_lance_attacks(&mut self) {
        for sq_idx in 0..81 {
            let sq = Square(sq_idx);
            let file = sq.file();
            let rank = sq.rank();
            
            // 先手の香: 前方直進
            let mut bb = Bitboard::ZERO;
            for r in 0..rank.to_u8() {
                bb.set(Square::new(file, Rank::from_u8(r).unwrap()));
            }
            // 全パターン（占有状態）で設定（簡易版: 障害物無視）
            for occ in 0..128 {
                self.lance[Color::Black as usize][sq_idx as usize][occ] = bb;
            }
            
            // 後手の香: 後方直進
            let mut bb = Bitboard::ZERO;
            for r in (rank.to_u8() + 1)..9 {
                bb.set(Square::new(file, Rank::from_u8(r).unwrap()));
            }
            for occ in 0..128 {
                self.lance[Color::White as usize][sq_idx as usize][occ] = bb;
            }
        }
    }
    
    /// 指定駒種の利きを取得
    pub fn attacks(&self, pt: PieceType, c: Color, sq: Square) -> Bitboard {
        let sq_idx = sq.0 as usize;
        let c_idx = c as usize;
        
        match pt {
            PieceType::Pawn => self.pawn[c_idx][sq_idx],
            PieceType::Lance => self.lance[c_idx][sq_idx][0], // 簡易版
            PieceType::Knight => self.knight[c_idx][sq_idx],
            PieceType::Silver => self.silver[c_idx][sq_idx],
            PieceType::Gold | PieceType::ProPawn | PieceType::ProLance |
            PieceType::ProKnight | PieceType::ProSilver => self.gold[c_idx][sq_idx],
            PieceType::King => self.king[sq_idx],
            PieceType::Bishop => self.bishop_attacks(sq, Bitboard::ZERO),
            PieceType::Rook => self.rook_attacks(sq, Bitboard::ZERO),
            PieceType::Horse => self.bishop_attacks(sq, Bitboard::ZERO) | self.king[sq_idx],
            PieceType::Dragon => self.rook_attacks(sq, Bitboard::ZERO) | self.king[sq_idx],
            _ => Bitboard::ZERO,
        }
    }
    
    /// 角の利き（簡易版: 障害物考慮なし）
    fn bishop_attacks(&self, sq: Square, _occupied: Bitboard) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        
        // 4方向の斜め
        let dirs = [SquareDelta::NE, SquareDelta::NW, SquareDelta::SE, SquareDelta::SW];
        
        for &dir in &dirs {
            let mut current = sq;
            while let Some(next) = current + dir {
                bb.set(next);
                current = next;
            }
        }
        
        bb
    }
    
    /// 飛の利き（簡易版: 障害物考慮なし）
    fn rook_attacks(&self, sq: Square, _occupied: Bitboard) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        
        // 4方向の縦横
        let dirs = [SquareDelta::N, SquareDelta::S, SquareDelta::E, SquareDelta::W];
        
        for &dir in &dirs {
            let mut current = sq;
            while let Some(next) = current + dir {
                bb.set(next);
                current = next;
            }
        }
        
        bb
    }
}

/// グローバル攻撃テーブル（遅延初期化）
use std::sync::OnceLock;
static ATTACK_TABLES: OnceLock<AttackTables> = OnceLock::new();

/// 攻撃テーブルを初期化
pub fn init_attack_tables() {
    ATTACK_TABLES.get_or_init(AttackTables::new);
}

/// 攻撃テーブルを取得
pub fn get_attack_tables() -> &'static AttackTables {
    ATTACK_TABLES.get_or_init(AttackTables::new)
}

/// 駒の利きを取得（便利関数）
pub fn attacks_from(pt: PieceType, c: Color, sq: Square) -> Bitboard {
    get_attack_tables().attacks(pt, c, sq)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attack_tables() {
        init_attack_tables();
        
        // 先手の歩の利き
        let sq = Square::new(File::File7, Rank::Rank7);
        let bb = attacks_from(PieceType::Pawn, Color::Black, sq);
        assert_eq!(bb.count(), 1);
        assert!(bb.is_set(Square::new(File::File7, Rank::Rank6)));
        
        // 先手の桂の利き
        let bb = attacks_from(PieceType::Knight, Color::Black, sq);
        assert_eq!(bb.count(), 2);
        
        // 金の利き
        let bb = attacks_from(PieceType::Gold, Color::Black, sq);
        assert_eq!(bb.count(), 6);
        
        // 玉の利き
        let sq = Square::new(File::File5, Rank::Rank5);
        let bb = attacks_from(PieceType::King, Color::Black, sq);
        assert_eq!(bb.count(), 8);
    }
}
