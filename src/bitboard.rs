// bitboard.rs - Bitboard実装（cshogiのbitboard.hpp/cppより）

use crate::types::{Square, File, Rank};

/// Bitboard - 81マスを128ビット（2つのu64）で表現
/// p0: Square 0-62 (1-7筋)
/// p1: Square 63-80 (8-9筋)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Bitboard {
    p0: u64,  // 下位63ビット
    p1: u64,  // 上位18ビット（実際は18ビットのみ使用）
}

impl Bitboard {
    /// 空のBitboard
    pub const ZERO: Bitboard = Bitboard { p0: 0, p1: 0 };
    
    /// 全てのマスが立っているBitboard
    pub const ALL: Bitboard = Bitboard { 
        p0: 0x7FFFFFFFFFFFFFFF,  // 63ビット全て
        p1: 0x3FFFF,              // 18ビット (81-63=18)
    };
    
    /// 新しいBitboardを作成
    #[inline]
    pub const fn new(p0: u64, p1: u64) -> Bitboard {
        Bitboard { p0, p1 }
    }
    
    /// 空かチェック
    #[inline]
    pub fn is_zero(self) -> bool {
        (self.p0 | self.p1) == 0
    }
    
    /// 少なくとも1ビット立っているかチェック
    #[inline]
    pub fn is_any(self) -> bool {
        !self.is_zero()
    }
    
    /// 指定したSquareのビットが立っているかチェック
    #[inline]
    pub fn is_set(self, sq: Square) -> bool {
        (self & Bitboard::square_mask(sq)).is_any()
    }
    
    /// 指定したSquareのビットを立てる
    #[inline]
    pub fn set(&mut self, sq: Square) {
        *self |= Bitboard::square_mask(sq);
    }
    
    /// 指定したSquareのビットを消す
    #[inline]
    pub fn clear(&mut self, sq: Square) {
        *self &= !Bitboard::square_mask(sq);
    }
    
    /// 指定したSquareのビットを反転
    #[inline]
    pub fn toggle(&mut self, sq: Square) {
        *self ^= Bitboard::square_mask(sq);
    }
    
    /// 最下位の1ビットを取得して消す
    #[inline]
    pub fn pop_first(&mut self) -> Option<Square> {
        if self.p0 != 0 {
            let sq = Square(self.p0.trailing_zeros() as u8);
            self.p0 &= self.p0 - 1;  // 最下位ビットを消す
            Some(sq)
        } else if self.p1 != 0 {
            let sq = Square(63 + self.p1.trailing_zeros() as u8);
            self.p1 &= self.p1 - 1;
            Some(sq)
        } else {
            None
        }
    }
    
    /// 最下位の1ビットを取得（消さない）
    #[inline]
    pub fn first(self) -> Option<Square> {
        if self.p0 != 0 {
            Some(Square(self.p0.trailing_zeros() as u8))
        } else if self.p1 != 0 {
            Some(Square(63 + self.p1.trailing_zeros() as u8))
        } else {
            None
        }
    }
    
    /// 立っているビット数をカウント
    #[inline]
    pub fn count(self) -> u32 {
        self.p0.count_ones() + self.p1.count_ones()
    }
    
    /// 指定したSquareのマスクを生成
    #[inline]
    pub fn square_mask(sq: Square) -> Bitboard {
        if sq.0 < 63 {
            Bitboard::new(1u64 << sq.0, 0)
        } else {
            Bitboard::new(0, 1u64 << (sq.0 - 63))
        }
    }
    
    /// File全体のマスク
    pub fn file_mask(f: File) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for r in 0..9 {
            bb.set(Square::new(f, Rank::from_u8(r).unwrap()));
        }
        bb
    }
    
    /// Rank全体のマスク
    pub fn rank_mask(r: Rank) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for f in 0..9 {
            bb.set(Square::new(File::from_u8(f).unwrap(), r));
        }
        bb
    }
    
    /// 先手から見た前方マスク（指定Rankより前）
    pub fn in_front_mask_black(r: Rank) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for rank in 0..r.to_u8() {
            bb |= Bitboard::rank_mask(Rank::from_u8(rank).unwrap());
        }
        bb
    }
    
    /// 後手から見た前方マスク（指定Rankより前）
    pub fn in_front_mask_white(r: Rank) -> Bitboard {
        let mut bb = Bitboard::ZERO;
        for rank in (r.to_u8() + 1)..9 {
            bb |= Bitboard::rank_mask(Rank::from_u8(rank).unwrap());
        }
        bb
    }
}

// ビット演算の実装
impl std::ops::Not for Bitboard {
    type Output = Bitboard;
    
    #[inline]
    fn not(self) -> Bitboard {
        Bitboard::new(!self.p0 & 0x7FFFFFFFFFFFFFFF, !self.p1 & 0x3FFFF)
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Bitboard;
    
    #[inline]
    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard::new(self.p0 & rhs.p0, self.p1 & rhs.p1)
    }
}

impl std::ops::BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Bitboard) {
        self.p0 &= rhs.p0;
        self.p1 &= rhs.p1;
    }
}

impl std::ops::BitOr for Bitboard {
    type Output = Bitboard;
    
    #[inline]
    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard::new(self.p0 | rhs.p0, self.p1 | rhs.p1)
    }
}

impl std::ops::BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Bitboard) {
        self.p0 |= rhs.p0;
        self.p1 |= rhs.p1;
    }
}

impl std::ops::BitXor for Bitboard {
    type Output = Bitboard;
    
    #[inline]
    fn bitxor(self, rhs: Bitboard) -> Bitboard {
        Bitboard::new(self.p0 ^ rhs.p0, self.p1 ^ rhs.p1)
    }
}

impl std::ops::BitXorAssign for Bitboard {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Bitboard) {
        self.p0 ^= rhs.p0;
        self.p1 ^= rhs.p1;
    }
}

impl std::fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Bitboard:")?;
        for rank in 0..9 {
            for file in (0..9).rev() {
                let sq = Square::new(
                    File::from_u8(file).unwrap(),
                    Rank::from_u8(rank).unwrap()
                );
                if self.is_set(sq) {
                    write!(f, "1 ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Bitboardのイテレータ
pub struct BitboardIter {
    bb: Bitboard,
}

impl Iterator for BitboardIter {
    type Item = Square;
    
    fn next(&mut self) -> Option<Square> {
        self.bb.pop_first()
    }
}

impl Bitboard {
    /// イテレータを作成
    pub fn iter(self) -> BitboardIter {
        BitboardIter { bb: self }
    }
    
    /// LSB（最下位ビット）を取得
    pub fn lsb(self) -> Option<Square> {
        self.first()
    }
    
    /// LSBを取り出して削除
    pub fn pop_lsb(&mut self) -> Square {
        self.pop_first().unwrap()
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;
    
    fn into_iter(self) -> BitboardIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bitboard_basic() {
        let mut bb = Bitboard::ZERO;
        assert!(bb.is_zero());
        
        let sq = Square::new(File::File7, Rank::Rank7);
        bb.set(sq);
        assert!(bb.is_set(sq));
        assert!(!bb.is_zero());
        
        bb.clear(sq);
        assert!(!bb.is_set(sq));
        assert!(bb.is_zero());
    }
    
    #[test]
    fn test_bitboard_count() {
        let mut bb = Bitboard::ZERO;
        assert_eq!(bb.count(), 0);
        
        bb.set(Square::new(File::File1, Rank::Rank1));
        bb.set(Square::new(File::File9, Rank::Rank9));
        assert_eq!(bb.count(), 2);
    }
    
    #[test]
    fn test_bitboard_iter() {
        let mut bb = Bitboard::ZERO;
        bb.set(Square::new(File::File1, Rank::Rank1));
        bb.set(Square::new(File::File5, Rank::Rank5));
        bb.set(Square::new(File::File9, Rank::Rank9));
        
        let squares: Vec<Square> = bb.into_iter().collect();
        assert_eq!(squares.len(), 3);
    }
    
    #[test]
    fn test_bitboard_operations() {
        let bb1 = Bitboard::square_mask(Square::new(File::File1, Rank::Rank1));
        let bb2 = Bitboard::square_mask(Square::new(File::File9, Rank::Rank9));
        
        let bb_or = bb1 | bb2;
        assert_eq!(bb_or.count(), 2);
        
        let bb_and = bb1 & bb2;
        assert!(bb_and.is_zero());
        
        let bb_not = !bb1;
        assert_eq!(bb_not.count(), 80);
    }

    /// LSB（最下位ビット）を取得
    pub fn lsb(self) -> Option<Square> {
        if self.is_zero() {
            return None;
        }
        
        if self.p0 != 0 {
            let idx = self.p0.trailing_zeros() as u8;
            Some(Square(idx))
        } else {
            let idx = 63 + self.p1.trailing_zeros() as u8;
            Some(Square(idx))
        }
    }
    
    /// LSBを取り出して削除
    pub fn pop_lsb(&mut self) -> Square {
        let sq = self.lsb().unwrap();
        *self &= *self - Bitboard::square_mask(sq);
        sq
    }
    
    /// イテレータ
    pub fn iter(self) -> BitboardIter {
        BitboardIter { bb: self }
    }
}

