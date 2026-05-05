// hand.rs - 持ち駒の管理（cshogiのhand.hppより）

use crate::piece::{HandPiece, PieceType};

/// 持ち駒（32ビットにパック）
/// cshogiと同じビットレイアウト:
/// - bits 0-4:   歩の枚数 (0-18)
/// - bits 6-8:   香の枚数 (0-4)
/// - bits 10-12: 桂の枚数 (0-4)
/// - bits 14-16: 銀の枚数 (0-4)
/// - bits 18-20: 金の枚数 (0-4)
/// - bits 22-23: 角の枚数 (0-2)
/// - bits 25-26: 飛の枚数 (0-2)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hand(u32);

impl Hand {
    pub const EMPTY: Hand = Hand(0);
    
    // ビットシフト量
    const PAWN_SHIFT: u32 = 0;
    const LANCE_SHIFT: u32 = 6;
    const KNIGHT_SHIFT: u32 = 10;
    const SILVER_SHIFT: u32 = 14;
    const GOLD_SHIFT: u32 = 18;
    const BISHOP_SHIFT: u32 = 22;
    const ROOK_SHIFT: u32 = 25;
    
    // マスク
    const PAWN_MASK: u32 = 0x1F << Self::PAWN_SHIFT;
    const LANCE_MASK: u32 = 0x7 << Self::LANCE_SHIFT;
    const KNIGHT_MASK: u32 = 0x7 << Self::KNIGHT_SHIFT;
    const SILVER_MASK: u32 = 0x7 << Self::SILVER_SHIFT;
    const GOLD_MASK: u32 = 0x7 << Self::GOLD_SHIFT;
    const BISHOP_MASK: u32 = 0x3 << Self::BISHOP_SHIFT;
    const ROOK_MASK: u32 = 0x3 << Self::ROOK_SHIFT;
    
    // 増減単位
    const PAWN_ONE: u32 = 1 << Self::PAWN_SHIFT;
    const LANCE_ONE: u32 = 1 << Self::LANCE_SHIFT;
    const KNIGHT_ONE: u32 = 1 << Self::KNIGHT_SHIFT;
    const SILVER_ONE: u32 = 1 << Self::SILVER_SHIFT;
    const GOLD_ONE: u32 = 1 << Self::GOLD_SHIFT;
    const BISHOP_ONE: u32 = 1 << Self::BISHOP_SHIFT;
    const ROOK_ONE: u32 = 1 << Self::ROOK_SHIFT;
    
    /// 新しい持ち駒を作成
    #[inline]
    pub fn new() -> Hand {
        Hand::EMPTY
    }
    
    /// 値から作成
    #[inline]
    pub fn from_u32(value: u32) -> Hand {
        Hand(value)
    }
    
    /// u32値を取得
    #[inline]
    pub fn value(self) -> u32 {
        self.0
    }
    
    /// 指定した駒の枚数を取得
    #[inline]
    pub fn count(self, hp: HandPiece) -> u32 {
        match hp {
            HandPiece::HPawn => (self.0 & Self::PAWN_MASK) >> Self::PAWN_SHIFT,
            HandPiece::HLance => (self.0 & Self::LANCE_MASK) >> Self::LANCE_SHIFT,
            HandPiece::HKnight => (self.0 & Self::KNIGHT_MASK) >> Self::KNIGHT_SHIFT,
            HandPiece::HSilver => (self.0 & Self::SILVER_MASK) >> Self::SILVER_SHIFT,
            HandPiece::HGold => (self.0 & Self::GOLD_MASK) >> Self::GOLD_SHIFT,
            HandPiece::HBishop => (self.0 & Self::BISHOP_MASK) >> Self::BISHOP_SHIFT,
            HandPiece::HRook => (self.0 & Self::ROOK_MASK) >> Self::ROOK_SHIFT,
        }
    }
    
    /// 指定した駒を持っているかチェック
    #[inline]
    pub fn exists(self, hp: HandPiece) -> bool {
        self.count(hp) > 0
    }
    
    /// 歩以外の駒を持っているかチェック
    #[inline]
    pub fn except_pawn_exists(self) -> bool {
        (self.0 & !Self::PAWN_MASK) != 0
    }
    
    /// 駒を1枚追加
    #[inline]
    pub fn add_one(&mut self, hp: HandPiece) {
        self.0 += match hp {
            HandPiece::HPawn => Self::PAWN_ONE,
            HandPiece::HLance => Self::LANCE_ONE,
            HandPiece::HKnight => Self::KNIGHT_ONE,
            HandPiece::HSilver => Self::SILVER_ONE,
            HandPiece::HGold => Self::GOLD_ONE,
            HandPiece::HBishop => Self::BISHOP_ONE,
            HandPiece::HRook => Self::ROOK_ONE,
        };
    }
    
    /// 駒を1枚削除
    #[inline]
    pub fn remove_one(&mut self, hp: HandPiece) {
        self.0 -= match hp {
            HandPiece::HPawn => Self::PAWN_ONE,
            HandPiece::HLance => Self::LANCE_ONE,
            HandPiece::HKnight => Self::KNIGHT_ONE,
            HandPiece::HSilver => Self::SILVER_ONE,
            HandPiece::HGold => Self::GOLD_ONE,
            HandPiece::HBishop => Self::BISHOP_ONE,
            HandPiece::HRook => Self::ROOK_ONE,
        };
    }
    
    /// 指定枚数を設定
    #[inline]
    pub fn set(&mut self, hp: HandPiece, count: u32) {
        let (mask, shift) = match hp {
            HandPiece::HPawn => (Self::PAWN_MASK, Self::PAWN_SHIFT),
            HandPiece::HLance => (Self::LANCE_MASK, Self::LANCE_SHIFT),
            HandPiece::HKnight => (Self::KNIGHT_MASK, Self::KNIGHT_SHIFT),
            HandPiece::HSilver => (Self::SILVER_MASK, Self::SILVER_SHIFT),
            HandPiece::HGold => (Self::GOLD_MASK, Self::GOLD_SHIFT),
            HandPiece::HBishop => (Self::BISHOP_MASK, Self::BISHOP_SHIFT),
            HandPiece::HRook => (Self::ROOK_MASK, Self::ROOK_SHIFT),
        };
        
        self.0 = (self.0 & !mask) | (count << shift);
    }
    
    /// SFEN形式の文字列に変換
    pub fn to_sfen(self) -> String {
        if self == Hand::EMPTY {
            return "-".to_string();
        }
        
        let mut s = String::new();
        
        // 順番: 飛角金銀桂香歩（価値の高い順）
        let order = [
            (HandPiece::HRook, 'R'),
            (HandPiece::HBishop, 'B'),
            (HandPiece::HGold, 'G'),
            (HandPiece::HSilver, 'S'),
            (HandPiece::HKnight, 'N'),
            (HandPiece::HLance, 'L'),
            (HandPiece::HPawn, 'P'),
        ];
        
        for (hp, ch) in &order {
            let count = self.count(*hp);
            if count > 0 {
                if count > 1 {
                    s.push_str(&count.to_string());
                }
                s.push(*ch);
            }
        }
        
        s
    }
    
    /// SFEN形式の文字列からパース
    pub fn from_sfen(s: &str) -> Option<Hand> {
        if s == "-" {
            return Some(Hand::EMPTY);
        }
        
        let mut hand = Hand::EMPTY;
        let mut chars = s.chars().peekable();
        
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
            
            let hp = match piece_char.to_uppercase().next()? {
                'P' => HandPiece::HPawn,
                'L' => HandPiece::HLance,
                'N' => HandPiece::HKnight,
                'S' => HandPiece::HSilver,
                'G' => HandPiece::HGold,
                'B' => HandPiece::HBishop,
                'R' => HandPiece::HRook,
                _ => return None,
            };
            
            hand.set(hp, count);
        }
        
        Some(hand)
    }
    
    /// PieceTypeから持ち駒を追加
    #[inline]
    pub fn add_piece(&mut self, pt: PieceType) {
        if let Some(hp) = HandPiece::from_piece_type(pt) {
            self.add_one(hp);
        }
    }
    
    /// PieceTypeから持ち駒を削除
    #[inline]
    pub fn remove_piece(&mut self, pt: PieceType) {
        if let Some(hp) = HandPiece::from_piece_type(pt) {
            self.remove_one(hp);
        }
    }
}

impl Default for Hand {
    fn default() -> Self {
        Hand::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hand_basic() {
        let mut hand = Hand::new();
        assert_eq!(hand, Hand::EMPTY);
        assert_eq!(hand.count(HandPiece::HPawn), 0);
        
        hand.add_one(HandPiece::HPawn);
        assert_eq!(hand.count(HandPiece::HPawn), 1);
        
        hand.add_one(HandPiece::HPawn);
        assert_eq!(hand.count(HandPiece::HPawn), 2);
        
        hand.remove_one(HandPiece::HPawn);
        assert_eq!(hand.count(HandPiece::HPawn), 1);
    }
    
    #[test]
    fn test_hand_multiple() {
        let mut hand = Hand::new();
        hand.add_one(HandPiece::HRook);
        hand.add_one(HandPiece::HBishop);
        hand.add_one(HandPiece::HPawn);
        hand.add_one(HandPiece::HPawn);
        
        assert_eq!(hand.count(HandPiece::HRook), 1);
        assert_eq!(hand.count(HandPiece::HBishop), 1);
        assert_eq!(hand.count(HandPiece::HPawn), 2);
        assert_eq!(hand.count(HandPiece::HGold), 0);
    }
    
    #[test]
    fn test_hand_sfen() {
        let mut hand = Hand::new();
        hand.set(HandPiece::HRook, 1);
        hand.set(HandPiece::HBishop, 1);
        hand.set(HandPiece::HPawn, 3);
        
        let sfen = hand.to_sfen();
        assert_eq!(sfen, "RB3P");
        
        let parsed = Hand::from_sfen(&sfen).unwrap();
        assert_eq!(hand, parsed);
    }
    
    #[test]
    fn test_hand_empty_sfen() {
        let hand = Hand::EMPTY;
        assert_eq!(hand.to_sfen(), "-");
        
        let parsed = Hand::from_sfen("-").unwrap();
        assert_eq!(hand, parsed);
    }
}
