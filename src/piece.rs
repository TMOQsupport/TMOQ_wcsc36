// piece.rs - 駒の定義（cshogiのpiece.hppより）

use crate::types::Color;

/// 駒の種類（先手・後手共通）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PieceType {
    Occupied = 0,  // 使用しない（Bitboard用）
    Pawn = 1,      // 歩
    Lance = 2,     // 香
    Knight = 3,    // 桂
    Silver = 4,    // 銀
    Bishop = 5,    // 角
    Rook = 6,      // 飛
    Gold = 7,      // 金
    King = 8,      // 玉
    ProPawn = 9,   // と
    ProLance = 10, // 成香
    ProKnight = 11,// 成桂
    ProSilver = 12,// 成銀
    Horse = 13,    // 馬
    Dragon = 14,   // 龍
}

impl PieceType {
    pub const NUM: usize = 15;
    
    /// 成れる駒かチェック
    #[inline]
    pub fn can_promote(self) -> bool {
        matches!(self, 
            PieceType::Pawn | PieceType::Lance | PieceType::Knight | 
            PieceType::Silver | PieceType::Bishop | PieceType::Rook
        )
    }
    
    /// 成った後の駒を返す
    #[inline]
    pub fn promote(self) -> Option<PieceType> {
        match self {
            PieceType::Pawn => Some(PieceType::ProPawn),
            PieceType::Lance => Some(PieceType::ProLance),
            PieceType::Knight => Some(PieceType::ProKnight),
            PieceType::Silver => Some(PieceType::ProSilver),
            PieceType::Bishop => Some(PieceType::Horse),
            PieceType::Rook => Some(PieceType::Dragon),
            _ => None,
        }
    }
    
    /// 成る前の駒を返す（成駒の場合）
    #[inline]
    pub fn unpromote(self) -> PieceType {
        match self {
            PieceType::ProPawn => PieceType::Pawn,
            PieceType::ProLance => PieceType::Lance,
            PieceType::ProKnight => PieceType::Knight,
            PieceType::ProSilver => PieceType::Silver,
            PieceType::Horse => PieceType::Bishop,
            PieceType::Dragon => PieceType::Rook,
            _ => self,
        }
    }
    
    /// 遠隔駒（飛角香）かチェック
    #[inline]
    pub fn is_slider(self) -> bool {
        matches!(self, 
            PieceType::Lance | PieceType::Bishop | PieceType::Rook |
            PieceType::Horse | PieceType::Dragon
        )
    }
    
    /// USI形式の文字列に変換
    pub fn to_usi(self) -> &'static str {
        match self {
            PieceType::Pawn => "P",
            PieceType::Lance => "L",
            PieceType::Knight => "N",
            PieceType::Silver => "S",
            PieceType::Bishop => "B",
            PieceType::Rook => "R",
            PieceType::Gold => "G",
            PieceType::King => "K",
            _ => "",
        }
    }
    
    /// SFEN形式の文字列に変換（成駒含む）
    pub fn to_sfen(self) -> &'static str {
        match self {
            PieceType::Pawn => "P",
            PieceType::Lance => "L",
            PieceType::Knight => "N",
            PieceType::Silver => "S",
            PieceType::Bishop => "B",
            PieceType::Rook => "R",
            PieceType::Gold => "G",
            PieceType::King => "K",
            PieceType::ProPawn => "+P",
            PieceType::ProLance => "+L",
            PieceType::ProKnight => "+N",
            PieceType::ProSilver => "+S",
            PieceType::Horse => "+B",
            PieceType::Dragon => "+R",
            _ => "",
        }
    }
}

/// 駒（先手・後手と種類を含む）
/// cshogiと同じエンコーディング:
/// - 下位4ビット: PieceType
/// - 5ビット目: Color (0=先手, 1=後手)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece(pub u8);

impl Piece {
    pub const EMPTY: Piece = Piece(0);
    pub const NUM: usize = 32;
    
    /// 先手の駒を作成
    #[inline]
    pub fn new_black(pt: PieceType) -> Piece {
        Piece(pt as u8)
    }
    
    /// 後手の駒を作成
    #[inline]
    pub fn new_white(pt: PieceType) -> Piece {
        Piece((pt as u8) | 0x10)
    }
    
    /// 色と駒種から作成
    #[inline]
    pub fn new(c: Color, pt: PieceType) -> Piece {
        Piece((pt as u8) | ((c as u8) << 4))
    }
    
    /// 空マスかチェック
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    
    /// 駒種を取得
    #[inline]
    pub fn piece_type(self) -> PieceType {
        unsafe { std::mem::transmute(self.0 & 0x0F) }
    }
    
    /// 色を取得
    #[inline]
    pub fn color(self) -> Color {
        if (self.0 & 0x10) == 0 {
            Color::Black
        } else {
            Color::White
        }
    }
    
    /// 反対色の駒に変換
    #[inline]
    pub fn inverse(self) -> Piece {
        Piece(self.0 ^ 0x10)
    }
    
    /// 成る
    #[inline]
    pub fn promote(self) -> Option<Piece> {
        self.piece_type().promote().map(|pt| Piece::new(self.color(), pt))
    }
    
    /// 成りを解除
    #[inline]
    pub fn unpromote(self) -> Piece {
        Piece::new(self.color(), self.piece_type().unpromote())
    }
    
    /// SFEN形式の文字列に変換
    pub fn to_sfen(self) -> String {
        if self.is_empty() {
            return String::new();
        }
        
        let pt_str = self.piece_type().to_sfen();
        match self.color() {
            Color::Black => pt_str.to_uppercase(),
            Color::White => pt_str.to_lowercase(),
        }
    }
    
    /// SFEN形式の文字からパース
    pub fn from_sfen_char(c: char) -> Option<Piece> {
        let (color, piece_char) = if c.is_uppercase() {
            (Color::Black, c)
        } else {
            (Color::White, c.to_uppercase().next()?)
        };
        
        let pt = match piece_char {
            'P' => PieceType::Pawn,
            'L' => PieceType::Lance,
            'N' => PieceType::Knight,
            'S' => PieceType::Silver,
            'B' => PieceType::Bishop,
            'R' => PieceType::Rook,
            'G' => PieceType::Gold,
            'K' => PieceType::King,
            _ => return None,
        };
        
        Some(Piece::new(color, pt))
    }
}

/// 持ち駒の種類（成駒は持てないので7種類のみ）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HandPiece {
    HPawn = 0,
    HLance = 1,
    HKnight = 2,
    HSilver = 3,
    HGold = 4,
    HBishop = 5,
    HRook = 6,
}

impl HandPiece {
    pub const NUM: usize = 7;
    
    /// PieceTypeに変換
    #[inline]
    pub fn to_piece_type(self) -> PieceType {
        match self {
            HandPiece::HPawn => PieceType::Pawn,
            HandPiece::HLance => PieceType::Lance,
            HandPiece::HKnight => PieceType::Knight,
            HandPiece::HSilver => PieceType::Silver,
            HandPiece::HGold => PieceType::Gold,
            HandPiece::HBishop => PieceType::Bishop,
            HandPiece::HRook => PieceType::Rook,
        }
    }
    
    /// PieceTypeから変換
    #[inline]
    pub fn from_piece_type(pt: PieceType) -> Option<HandPiece> {
        match pt.unpromote() {
            PieceType::Pawn => Some(HandPiece::HPawn),
            PieceType::Lance => Some(HandPiece::HLance),
            PieceType::Knight => Some(HandPiece::HKnight),
            PieceType::Silver => Some(HandPiece::HSilver),
            PieceType::Gold => Some(HandPiece::HGold),
            PieceType::Bishop => Some(HandPiece::HBishop),
            PieceType::Rook => Some(HandPiece::HRook),
            _ => None,
        }
    }
    
    /// USI形式の文字列に変換
    pub fn to_usi(self) -> &'static str {
        self.to_piece_type().to_usi()
    }
}

// 駒の価値（Apery駒点 WCSC26）
pub const PIECE_VALUE: [i32; 15] = [
    0,    // Occupied
    90,   // Pawn
    315,  // Lance
    405,  // Knight
    495,  // Silver
    855,  // Bishop
    990,  // Rook
    540,  // Gold
    15000,// King
    540,  // ProPawn
    540,  // ProLance
    540,  // ProKnight
    540,  // ProSilver
    1305, // Horse  (旧945 → 新1305: +450, 約53%増し)
    1530, // Dragon (旧1395 → 新1530: +540, 約55%増し)
];

#[inline]
pub fn piece_value(pt: PieceType) -> i32 {
    PIECE_VALUE[pt as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_piece_creation() {
        let p = Piece::new(Color::Black, PieceType::Pawn);
        assert_eq!(p.color(), Color::Black);
        assert_eq!(p.piece_type(), PieceType::Pawn);
    }
    
    #[test]
    fn test_piece_promote() {
        let pawn = Piece::new(Color::Black, PieceType::Pawn);
        let promoted = pawn.promote().unwrap();
        assert_eq!(promoted.piece_type(), PieceType::ProPawn);
    }
    
    #[test]
    fn test_piece_inverse() {
        let black_pawn = Piece::new(Color::Black, PieceType::Pawn);
        let white_pawn = black_pawn.inverse();
        assert_eq!(white_pawn.color(), Color::White);
        assert_eq!(white_pawn.piece_type(), PieceType::Pawn);
    }
}
