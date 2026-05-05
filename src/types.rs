// types.rs - 基本型定義（cshogiのsquare.hpp, color.hppより）

/// 先手・後手
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,  // 先手
    White = 1,  // 後手
}

impl Color {
    #[inline]
    pub fn opposite(self) -> Color {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
    
    #[inline]
    pub fn from_u8(v: u8) -> Option<Color> {
        match v {
            0 => Some(Color::Black),
            1 => Some(Color::White),
            _ => None,
        }
    }
}

// ! 演算子で opposite を呼び出せるようにする
impl std::ops::Not for Color {
    type Output = Color;
    
    fn not(self) -> Color {
        self.opposite()
    }
}

/// 筋（1-9）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum File {
    File1 = 0, File2, File3, File4, File5, File6, File7, File8, File9,
}

impl File {
    pub const NUM: usize = 9;
    
    #[inline]
    pub fn from_u8(v: u8) -> Option<File> {
        if v < 9 {
            unsafe { Some(std::mem::transmute(v)) }
        } else {
            None
        }
    }
    
    #[inline]
    pub fn to_u8(self) -> u8 {
        self as u8
    }
    
    #[inline]
    pub fn to_char(self) -> char {
        (b'1' + self as u8) as char
    }
    
    /// 加算（境界チェック付き）
    #[inline]
    pub fn add(self, delta: i32) -> Option<File> {
        let new_val = (self as u8 as i32) + delta;
        if new_val >= 0 && new_val < 9 {
            File::from_u8(new_val as u8)
        } else {
            None
        }
    }
    
    /// 減算（境界チェック付き）
    #[inline]
    pub fn sub(self, delta: i32) -> Option<File> {
        self.add(-delta)
    }
}

/// 段（1-9）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rank {
    Rank1 = 0, Rank2, Rank3, Rank4, Rank5, Rank6, Rank7, Rank8, Rank9,
}

impl Rank {
    pub const NUM: usize = 9;
    
    #[inline]
    pub fn from_u8(v: u8) -> Option<Rank> {
        if v < 9 {
            unsafe { Some(std::mem::transmute(v)) }
        } else {
            None
        }
    }
    
    #[inline]
    pub fn to_u8(self) -> u8 {
        self as u8
    }
    
    #[inline]
    pub fn to_char(self) -> char {
        (b'a' + self as u8) as char
    }
    
    /// 加算（境界チェック付き）
    #[inline]
    pub fn add(self, delta: i32) -> Option<Rank> {
        let new_val = (self as u8 as i32) + delta;
        if new_val >= 0 && new_val < 9 {
            Rank::from_u8(new_val as u8)
        } else {
            None
        }
    }
    
    /// 減算（境界チェック付き）
    #[inline]
    pub fn sub(self, delta: i32) -> Option<Rank> {
        self.add(-delta)
    }
}

/// マス目（0-80）
/// cshogiと同じレイアウト: file * 9 + rank
/// SQ11 = 0, SQ12 = 1, ..., SQ19 = 8, SQ21 = 9, ..., SQ99 = 80
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square(pub u8);

impl Square {
    pub const NUM: usize = 81;
    
    // 定数定義（cshogiのSquare enumと同じ）
    pub const SQ11: Square = Square(0);
    pub const SQ99: Square = Square(80);
    
    #[inline]
    pub fn new(file: File, rank: Rank) -> Square {
        Square(file.to_u8() * 9 + rank.to_u8())
    }
    
    #[inline]
    pub fn from_u8(v: u8) -> Option<Square> {
        if v < 81 {
            Some(Square(v))
        } else {
            None
        }
    }
    
    #[inline]
    pub fn file(self) -> File {
        File::from_u8(self.0 / 9).unwrap()
    }
    
    #[inline]
    pub fn rank(self) -> Rank {
        Rank::from_u8(self.0 % 9).unwrap()
    }
    
    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 < 81
    }
    
    /// USI形式の文字列に変換（例: "7g"）
    pub fn to_usi(self) -> String {
        format!("{}{}", self.file().to_char(), self.rank().to_char())
    }
    
    /// USI形式の文字列からパース（例: "7g" → Square）
    pub fn from_usi(s: &str) -> Option<Square> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 2 {
            return None;
        }
        
        let file = chars[0].to_digit(10)? as u8;
        let rank = (chars[1] as u8).wrapping_sub(b'a');
        
        if file == 0 || file > 9 || rank >= 9 {
            return None;
        }
        
        Some(Square::new(
            File::from_u8(file - 1)?,
            Rank::from_u8(rank)?
        ))
    }
    
    /// 成れる位置かチェック（敵陣）
    #[inline]
    pub fn can_promote(self, c: Color) -> bool {
        match c {
            Color::Black => self.rank() <= Rank::Rank3,
            Color::White => self.rank() >= Rank::Rank7,
        }
    }
}

/// Square間の差分
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquareDelta(pub i8);

impl SquareDelta {
    pub const N: SquareDelta = SquareDelta(-1);   // 上
    pub const S: SquareDelta = SquareDelta(1);    // 下
    pub const E: SquareDelta = SquareDelta(-9);   // 右
    pub const W: SquareDelta = SquareDelta(9);    // 左
    pub const NE: SquareDelta = SquareDelta(-10); // 右上
    pub const NW: SquareDelta = SquareDelta(8);   // 左上
    pub const SE: SquareDelta = SquareDelta(-8);  // 右下
    pub const SW: SquareDelta = SquareDelta(10);  // 左下
}

impl std::ops::Add<SquareDelta> for Square {
    type Output = Option<Square>;
    
    fn add(self, delta: SquareDelta) -> Option<Square> {
        let new_sq = self.0 as i8 + delta.0;
        
        // 範囲チェック
        if new_sq < 0 || new_sq >= 81 {
            return None;
        }
        
        let old_file = self.file();
        let old_rank = self.rank();
        let new_square = Square(new_sq as u8);
        let new_file = new_square.file();
        let new_rank = new_square.rank();
        
        // ファイル/ランクが1つだけ変わるか、両方変わる（斜め）かチェック
        let file_diff = (new_file.to_u8() as i8 - old_file.to_u8() as i8).abs();
        let rank_diff = (new_rank.to_u8() as i8 - old_rank.to_u8() as i8).abs();
        
        // 大きくジャンプ（折り返し）は無効
        if file_diff > 1 || rank_diff > 1 {
            return None;
        }
        
        Some(new_square)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_square_creation() {
        let sq = Square::new(File::File7, Rank::Rank7);
        assert_eq!(sq.file(), File::File7);
        assert_eq!(sq.rank(), Rank::Rank7);
    }
    
    #[test]
    fn test_usi_conversion() {
        let sq = Square::new(File::File7, Rank::Rank7);
        assert_eq!(sq.to_usi(), "7g");
        
        let sq2 = Square::from_usi("7g").unwrap();
        assert_eq!(sq, sq2);
    }
    
    #[test]
    fn test_can_promote() {
        let sq1 = Square::new(File::File7, Rank::Rank3);
        assert!(sq1.can_promote(Color::Black));
        assert!(!sq1.can_promote(Color::White));
        
        let sq2 = Square::new(File::File3, Rank::Rank7);
        assert!(!sq2.can_promote(Color::Black));
        assert!(sq2.can_promote(Color::White));
    }
    
    #[test]
    fn test_square_delta() {
        // 9筋（右端）から東（右）に進めない
        let sq = Square::new(File::File9, Rank::Rank5);
        assert_eq!(sq + SquareDelta::E, None);
        
        // 1筋（左端）から西（左）に進めない
        let sq = Square::new(File::File1, Rank::Rank5);
        assert_eq!(sq + SquareDelta::W, None);
        
        // 1段（上端）から北（上）に進めない
        let sq = Square::new(File::File5, Rank::Rank1);
        assert_eq!(sq + SquareDelta::N, None);
        
        // 9段（下端）から南（下）に進めない
        let sq = Square::new(File::File5, Rank::Rank9);
        assert_eq!(sq + SquareDelta::S, None);
        
        // 正常な移動
        let sq = Square::new(File::File5, Rank::Rank5);
        let sq_n = (sq + SquareDelta::N).unwrap();
        assert_eq!(sq_n, Square::new(File::File5, Rank::Rank4));
    }
}
