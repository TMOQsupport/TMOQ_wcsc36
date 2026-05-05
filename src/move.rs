// move.rs - 指し手の定義（cshogiのmove.hppより）

use crate::types::Square;
use crate::piece::{PieceType, HandPiece};

/// 指し手（32ビットにパック）
/// ビットレイアウト（cshogiと同じ）:
/// - bits 0-6:   移動先 (to)
/// - bits 7-13:  移動元 (from) / 駒打ちの場合は PieceType + 80
/// - bit 14:     成りフラグ
/// - bits 16-19: 移動する駒の種類
/// - bits 20-23: 取った駒の種類
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move(u32);

impl Move {
    pub const PROMOTE_FLAG: u32 = 1 << 14;
    pub const NONE: Move = Move(0);
    pub const NULL: Move = Move(129);
    
    /// 新しい指し手を作成（通常の移動）
    pub fn new_normal(from: Square, to: Square, pt: PieceType, promote: bool) -> Move {
        let mut value = (to.0 as u32) | ((from.0 as u32) << 7) | ((pt as u32) << 16);
        if promote {
            value |= Self::PROMOTE_FLAG;
        }
        Move(value)
    }
    
    /// 新しい指し手を作成（駒打ち）
    pub fn new_drop(pt: PieceType, to: Square) -> Move {
        let from_encoded = 80 + pt as u32;  // 81以上で駒打ち
        Move((to.0 as u32) | (from_encoded << 7))
    }
    
    /// 駒を取る手を作成
    pub fn with_capture(mut self, captured: PieceType) -> Move {
        self.0 |= (captured as u32) << 20;
        self
    }
    
    /// 移動先を取得
    #[inline]
    pub fn to(self) -> Square {
        Square((self.0 & 0x7F) as u8)
    }
    
    /// 指し手のビット値全体（置換表保存用）
    #[inline]
    pub fn raw(self) -> u32 { self.0 }

    /// ビット値から Move を復元（0 なら None）
    #[inline]
    pub fn from_raw(bits: u32) -> Option<Move> {
        if bits == 0 { None } else { Some(Move(bits)) }
    }

    /// 移動元のビット（駒打ちなら81以上）
    #[inline]
    pub fn from_bits(self) -> u8 {
        ((self.0 >> 7) & 0x7F) as u8
    }
    
    /// 移動元を取得（駒打ちの場合はNone）
    #[inline]
    pub fn from(self) -> Option<Square> {
        let from = self.from_bits();
        if from < 81 {
            Some(Square(from))
        } else {
            None
        }
    }
    
    /// 駒打ちかチェック
    #[inline]
    pub fn is_drop(self) -> bool {
        self.from_bits() >= 81
    }
    
    /// 成りかチェック
    #[inline]
    pub fn is_promotion(self) -> bool {
        (self.0 & Self::PROMOTE_FLAG) != 0
    }
    
    /// 駒を取る手かチェック
    #[inline]
    pub fn is_capture(self) -> bool {
        (self.0 & 0xF00000) != 0
    }
    
    /// 移動する駒の種類
    #[inline]
    pub fn piece_type_from(self) -> PieceType {
        unsafe { std::mem::transmute(((self.0 >> 16) & 0xF) as u8) }
    }
    
    /// 移動後の駒の種類
    #[inline]
    pub fn piece_type_to(self) -> PieceType {
        if self.is_drop() {
            self.piece_type_dropped()
        } else {
            let pt = self.piece_type_from();
            if self.is_promotion() {
                pt.promote().unwrap_or(pt)
            } else {
                pt
            }
        }
    }
    
    /// 打つ駒の種類
    #[inline]
    pub fn piece_type_dropped(self) -> PieceType {
        unsafe { std::mem::transmute((self.from_bits() - 80) as u8) }
    }
    
    /// 取った駒の種類
    #[inline]
    pub fn captured(self) -> Option<PieceType> {
        let cap = ((self.0 >> 20) & 0xF) as u8;
        if cap == 0 {
            None
        } else {
            Some(unsafe { std::mem::transmute(cap) })
        }
    }
    
    /// 打つ駒の持ち駒型
    #[inline]
    pub fn hand_piece_dropped(self) -> Option<HandPiece> {
        if self.is_drop() {
            HandPiece::from_piece_type(self.piece_type_dropped())
        } else {
            None
        }
    }
    
    /// USI形式の文字列に変換
    pub fn to_usi(self) -> String {
        if self == Move::NONE {
            return "none".to_string();
        }
        if self == Move::NULL {
            return "null".to_string();
        }
        
        let to_str = self.to().to_usi();
        
        if self.is_drop() {
            // 駒打ち: "P*5e"
            let pt = self.piece_type_dropped();
            format!("{}*{}", pt.to_usi(), to_str)
        } else {
            // 通常の移動: "7g7f" or "2b2a+"
            let from_str = self.from().unwrap().to_usi();
            if self.is_promotion() {
                format!("{}{}+", from_str, to_str)
            } else {
                format!("{}{}", from_str, to_str)
            }
        }
    }
    
    /// USIで比較
    pub fn matches_usi(self, usi: &str) -> bool {
        self.to_usi() == usi
    }
    
    /// USI形式の文字列からパース
    pub fn from_usi(s: &str) -> Option<Move> {
        if s == "none" {
            return Some(Move::NONE);
        }
        if s == "null" {
            return Some(Move::NULL);
        }
        
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 4 {
            return None;
        }
        
        // 駒打ち: "P*5e"
        if chars.len() >= 4 && chars[1] == '*' {
            let pt = match chars[0] {
                'P' => PieceType::Pawn,
                'L' => PieceType::Lance,
                'N' => PieceType::Knight,
                'S' => PieceType::Silver,
                'G' => PieceType::Gold,
                'B' => PieceType::Bishop,
                'R' => PieceType::Rook,
                _ => return None,
            };
            
            let to_str: String = chars[2..].iter().collect();
            let to = Square::from_usi(&to_str)?;
            
            return Some(Move::new_drop(pt, to));
        }
        
        // 通常の移動: "7g7f" or "7g7f+"
        if chars.len() >= 4 {
            let from_str: String = chars[0..2].iter().collect();
            let to_start = 2;
            let promote = chars.last() == Some(&'+');
            let to_end = if promote { chars.len() - 1 } else { chars.len() };
            
            if to_end - to_start < 2 {
                return None;
            }
            
            let to_str: String = chars[to_start..to_end].iter().collect();
            
            let from = Square::from_usi(&from_str)?;
            let to = Square::from_usi(&to_str)?;
            
            // PieceTypeは後で設定する必要がある（盤面情報が必要）
            // ここでは仮の値を入れる
            return Some(Move::new_normal(from, to, PieceType::Pawn, promote));
        }
        
        None
    }
    
    /// 有効な手かチェック
    #[inline]
    pub fn is_ok(self) -> bool {
        self != Move::NONE && self != Move::NULL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{File, Rank};
    
    #[test]
    fn test_move_normal() {
        let from = Square::new(File::File7, Rank::Rank7);
        let to = Square::new(File::File7, Rank::Rank6);
        let mv = Move::new_normal(from, to, PieceType::Pawn, false);
        
        assert_eq!(mv.from(), Some(from));
        assert_eq!(mv.to(), to);
        assert!(!mv.is_drop());
        assert!(!mv.is_promotion());
        assert_eq!(mv.piece_type_from(), PieceType::Pawn);
    }
    
    #[test]
    fn test_move_promotion() {
        let from = Square::new(File::File2, Rank::Rank2);
        let to = Square::new(File::File2, Rank::Rank1);
        let mv = Move::new_normal(from, to, PieceType::Pawn, true);
        
        assert!(mv.is_promotion());
        assert_eq!(mv.piece_type_to(), PieceType::ProPawn);
    }
    
    #[test]
    fn test_move_drop() {
        let to = Square::new(File::File5, Rank::Rank5);
        let mv = Move::new_drop(PieceType::Pawn, to);
        
        assert!(mv.is_drop());
        assert_eq!(mv.to(), to);
        assert_eq!(mv.piece_type_dropped(), PieceType::Pawn);
        assert_eq!(mv.from(), None);
    }
    
    #[test]
    fn test_move_usi() {
        // 通常の移動
        let from = Square::new(File::File7, Rank::Rank7);
        let to = Square::new(File::File7, Rank::Rank6);
        let mv = Move::new_normal(from, to, PieceType::Pawn, false);
        assert_eq!(mv.to_usi(), "7g7f");
        
        // 成る手
        let mv2 = Move::new_normal(from, to, PieceType::Pawn, true);
        assert_eq!(mv2.to_usi(), "7g7f+");
        
        // 駒打ち
        let mv3 = Move::new_drop(PieceType::Pawn, to);
        assert_eq!(mv3.to_usi(), "P*7f");
    }
    
    #[test]
    fn test_move_parse_usi() {
        // 通常の移動
        let mv = Move::from_usi("7g7f").unwrap();
        assert!(!mv.is_drop());
        assert_eq!(mv.to(), Square::new(File::File7, Rank::Rank6));
        
        // 成る手
        let mv2 = Move::from_usi("7g7f+").unwrap();
        assert!(mv2.is_promotion());
        
        // 駒打ち
        let mv3 = Move::from_usi("P*5e").unwrap();
        assert!(mv3.is_drop());
        assert_eq!(mv3.piece_type_dropped(), PieceType::Pawn);
    }
}
