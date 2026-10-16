//! Canonical visible-binary source boundary for SENS .lisp files.
//!
//! This module owns source token shape only. It deliberately does not assign
//! D1-D9 meaning; semantic wrappers consume these exact-width words later.

use crate::{
    Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9, ErrorKind, LanguageError, Span,
};
use std::fmt;

/// Canonical SENS source extension. The extension identifies the language;
/// exact-width binary words carry program meaning.
pub const CANONICAL_SOURCE_EXTENSION: &str = ".lisp";

/// One already-bounded visible-binary source word.
///
/// Width is preserved in the variant, so equal packed numeric values from
/// different source widths remain distinct: 1 != 01 != 001.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BinarySourceWord {
    W1(Bit1),
    W2(Bit2),
    W3(Bit3),
    W4(Bit4),
    W5(Bit5),
    W6(Bit6),
    W7(Bit7),
    W8(Bit8),
    W9(Bit9),
    W10(crate::DomainD10),
}

impl BinarySourceWord {
    pub const fn width(self) -> usize {
        match self {
            Self::W1(_) => Bit1::width(),
            Self::W2(_) => Bit2::width(),
            Self::W3(_) => Bit3::width(),
            Self::W4(_) => Bit4::width(),
            Self::W5(_) => Bit5::width(),
            Self::W6(_) => Bit6::width(),
            Self::W7(_) => Bit7::width(),
            Self::W8(_) => Bit8::width(),
            Self::W9(_) => Bit9::width(),
            Self::W10(_) => DomainD10::width(),
        }
    }

    /// Mechanical payload only. Width must remain attached to the enum value.
    ///
    /// `u16` is required so W9 can never be truncated to the historical byte lane.
    pub const fn packed_bits(self) -> u16 {
        match self {
            Self::W1(word) => word.packed_bits() as u16,
            Self::W2(word) => word.packed_bits() as u16,
            Self::W3(word) => word.packed_bits() as u16,
            Self::W4(word) => word.packed_bits() as u16,
            Self::W5(word) => word.packed_bits() as u16,
            Self::W6(word) => word.packed_bits() as u16,
            Self::W7(word) => word.packed_bits() as u16,
            Self::W8(word) => word.packed_bits() as u16,
            Self::W9(word) => word.packed_bits(),
            Self::W10(word) => word.packed_bits(),
        }
    }

    /// Lift an exact one-bit source word into the typed D1 carrier.
    ///
    /// This assigns no bit orientation: NO/YES remains language-owned.
    pub const fn d1(self) -> Option<crate::PredicateBit> {
        match self {
            Self::W1(word) => Some(crate::PredicateBit::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact two-bit source word into the typed D2 carrier.
    ///
    /// Structural roles remain language-owned; width is the only fact here.
    pub const fn d2(self) -> Option<crate::Racana2> {
        match self {
            Self::W2(word) => Some(crate::Racana2::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact three-bit source word into the typed D3 carrier.
    ///
    /// Primitive roles remain language-owned; no historical flat-byte mapping is
    /// performed at this boundary.
    pub const fn d3(self) -> Option<crate::Bija3> {
        match self {
            Self::W3(word) => Some(crate::Bija3::from_word(word)),
            _ => None,
        }
    }


    /// Lift an exact four-bit source word into the typed Core.D4 carrier.
    ///
    /// Occupancy and operation meaning remain owned by the D4 contract.
    pub const fn d4(self) -> Option<crate::CoreD4> {
        match self {
            Self::W4(word) => Some(crate::CoreD4::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact five-bit source word into the owner-ratified D5 carrier (#3305).
    pub const fn d5(self) -> Option<crate::CoreD5> {
        match self {
            Self::W5(word) => Some(crate::CoreD5::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact six-bit source word into the owner-ratified D6 carrier (#3393).
    pub const fn d6(self) -> Option<crate::CoreD6> {
        match self {
            Self::W6(word) => Some(crate::CoreD6::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact seven-bit source word into the typed D7 carrier.
    /// Contract 11.8 retains D7 residency under #3572, but exact width alone
    /// still does not select a D7 role or grant generic Core callability.
    pub const fn d7(self) -> Option<crate::D7DataWord> {
        match self {
            Self::W7(word) => Some(crate::D7DataWord::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact eight-bit source word into the owner-ratified D8 carrier (#3960).
    /// This never converts through historical flat-byte compatibility identity;
    /// runtime callability remains a separate mechanism decision.
    pub const fn d8(self) -> Option<crate::CoreD8> {
        match self {
            Self::W8(word) => Some(crate::CoreD8::from_word(word)),
            _ => None,
        }
    }

    /// Lift an exact nine-bit source word into the owner-ratified D9 carrier (#4008).
    /// Runtime callability remains a separate mechanism decision.
    pub const fn d9(self) -> Option<crate::CoreD9> {
        match self {
            Self::W9(word) => Some(crate::CoreD9::from_word(word)),
            Self::W10(word) => Some(crate::DomainD10::from_word(word)),
            _ => None,
        }
    }

    /// Total exact-width qualified lift for the W1→W9 carrier ladder.
    ///
    /// Contract 11.8: D1-D9 have current semantic authority.
    /// D1-D9 identity is materialized exactly; callability remains separate.
    pub const fn domain_identity(self) -> crate::DomainIdentity {
        crate::DomainIdentity::from_source_word(self)
    }
}

impl From<crate::PredicateBit> for BinarySourceWord {
    fn from(value: crate::PredicateBit) -> Self {
        Self::W1(value.word())
    }
}

impl From<crate::Racana2> for BinarySourceWord {
    fn from(value: crate::Racana2) -> Self {
        Self::W2(value.word())
    }
}

impl From<crate::Bija3> for BinarySourceWord {
    fn from(value: crate::Bija3) -> Self {
        Self::W3(value.word())
    }
}


impl From<crate::CoreD4> for BinarySourceWord {
    fn from(value: crate::CoreD4) -> Self {
        Self::W4(value.word())
    }
}

impl From<crate::CoreD5> for BinarySourceWord {
    fn from(value: crate::CoreD5) -> Self {
        Self::W5(value.word())
    }
}

impl From<crate::CoreD6> for BinarySourceWord {
    fn from(value: crate::CoreD6) -> Self {
        Self::W6(value.word())
    }
}

impl From<crate::D7DataWord> for BinarySourceWord {
    fn from(value: crate::D7DataWord) -> Self {
        Self::W7(value.word())
    }
}

impl From<crate::CoreD8> for BinarySourceWord {
    fn from(value: crate::CoreD8) -> Self {
        Self::W8(value.word())
    }
}

impl From<crate::CoreD9> for BinarySourceWord {
    fn from(value: crate::CoreD9) -> Self {
        Self::W9(value.word())
    }
}

impl fmt::Display for BinarySourceWord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:0width$b}",
            self.packed_bits(),
            width = self.width()
        )
    }
}

impl fmt::Debug for BinarySourceWord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "BinarySourceWord<{}>({self})", self.width())
    }
}

/// One canonical binary word together with its exact byte span in UTF-8 source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinarySourceToken {
    pub word: BinarySourceWord,
    pub span: Span,
}

/// Tokenize canonical visible-binary SENS source.
///
/// Whitespace separates words. Semicolon starts a comment through end-of-line.
/// Every non-comment token must contain only 0/1 and have width 1..=9.
/// This function performs no semantic lookup and no human-name fallback.
pub fn parse_binary_source_words(source: &str) -> Result<Vec<BinarySourceToken>, LanguageError> {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    let mut tokens = Vec::new();

    while cursor < bytes.len() {
        match bytes[cursor] {
            b if b.is_ascii_whitespace() => cursor += 1,
            b';' => {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            _ => {
                let start = cursor;
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && bytes[cursor] != b';'
                {
                    cursor += 1;
                }
                let token = &source[start..cursor];
                let word = parse_word(token).ok_or_else(|| {
                    let message = if token.len() > 9
                        && token.bytes().all(|byte| matches!(byte, b'0' | b'1'))
                    {
                        "canonical SENS source word exceeds current 9-bit carrier"
                    } else {
                        "canonical SENS source accepts only exact-width binary words"
                    };
                    LanguageError::new(
                        ErrorKind::Parse,
                        message,
                        Span { start, end: cursor },
                    )
                })?;
                tokens.push(BinarySourceToken {
                    word,
                    span: Span { start, end: cursor },
                });
            }
        }
    }

    Ok(tokens)
}

/// Один бінарний source-токен ратифікованої драбини D1–D10.
///
/// Тут є лише координата (ширина, біти), а не допуск до виконання.
/// Зокрема, D8:10101000 ніколи не отримує значення історичного W8.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainLadderSourceToken {
    pub coordinate: crate::domain_ladder::DomainCoordinate,
    pub span: Span,
}

/// Читає D1–D10 без обмеження старою 8-бітною таблицею.
///
/// Історичний parse_binary_source_words лишається незмінним для своїх
/// споживачів D1–D9. Новий механічний шлях зберігає точну ширину й
/// ведучі нулі, але сам собою не дає D10 callable-закону або D24+ числа.
pub fn parse_domain_ladder_source_words(
    source: &str,
) -> Result<Vec<DomainLadderSourceToken>, LanguageError> {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    let mut result = Vec::new();
    while cursor < bytes.len() {
        match bytes[cursor] {
            byte if byte.is_ascii_whitespace() => cursor += 1,
            b';' => {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            _ => {
                let start = cursor;
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && bytes[cursor] != b';'
                {
                    cursor += 1;
                }
                let token = &bytes[start..cursor];
                if !(1..=10).contains(&token.len())
                    || !token.iter().all(|&byte| matches!(byte, b'0' | b'1'))
                {
                    return Err(LanguageError::new(
                        ErrorKind::Parse,
                        "драбина D1–D10 приймає лише точні бінарні слова шириною 1–10",
                        Span { start, end: cursor },
                    ));
                }
                let mut bits = 0u16;
                for &bit in token {
                    bits = (bits << 1) | u16::from(bit - b'0');
                }
                let coordinate = crate::domain_ladder::DomainCoordinate::new(
                    token.len() as u8, bits,
                )
                .expect("перевірені 1–10 бітів утворюють механічну координату");
                result.push(DomainLadderSourceToken {
                    coordinate,
                    span: Span { start, end: cursor },
                });
            }
        }
    }
    Ok(result)
}


fn parse_word(token: &str) -> Option<BinarySourceWord> {
    if token.is_empty() || token.len() > 9 {
        return None;
    }
    if !token.bytes().all(|byte| matches!(byte, b'0' | b'1')) {
        return None;
    }

    let mut packed = 0u16;
    for byte in token.bytes() {
        packed = (packed << 1) | u16::from(byte - b'0');
    }

    let small = || u8::try_from(packed).ok();
    Some(match token.len() {
        1 => BinarySourceWord::W1(Bit1::new(small()?)?),
        2 => BinarySourceWord::W2(Bit2::new(small()?)?),
        3 => BinarySourceWord::W3(Bit3::new(small()?)?),
        4 => BinarySourceWord::W4(Bit4::new(small()?)?),
        5 => BinarySourceWord::W5(Bit5::new(small()?)?),
        6 => BinarySourceWord::W6(Bit6::new(small()?)?),
        7 => BinarySourceWord::W7(Bit7::new(small()?)?),
        8 => BinarySourceWord::W8(Bit8::new(small()?)?),
        9 => BinarySourceWord::W9(Bit9::new(packed)?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_example_preserves_exact_word_boundaries() {
        let tokens = parse_binary_source_words("10 001 01").expect("canonical source");
        let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
        let spellings: Vec<_> = tokens.iter().map(|token| token.word.to_string()).collect();

        assert_eq!(widths, vec![2, 3, 2]);
        assert_eq!(spellings, vec!["10", "001", "01"]);
        assert_eq!(tokens[0].span, Span { start: 0, end: 2 });
        assert_eq!(tokens[1].span, Span { start: 3, end: 6 });
        assert_eq!(tokens[2].span, Span { start: 7, end: 9 });
    }

    #[test]
    fn w9_source_token_preserves_high_bit_and_exact_width() {
        let tokens = parse_binary_source_words("000000001 100000001 111111111").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.word.width()).collect::<Vec<_>>(),
            [9, 9, 9]
        );
        assert_eq!(tokens[0].word.packed_bits(), 1);
        assert_eq!(tokens[1].word.packed_bits(), 257);
        assert_eq!(tokens[2].word.packed_bits(), 511);
        assert_eq!(tokens[1].word.to_string(), "100000001");
    }

    #[test]
    fn w9_does_not_alias_same_low_byte_w8() {
        let tokens = parse_binary_source_words("00000001 100000001").unwrap();
        assert_eq!(tokens[0].word.packed_bits(), 1);
        assert_eq!(tokens[1].word.packed_bits(), 257);
        assert_ne!(tokens[0].word, tokens[1].word);
        assert!(tokens[0].word.d9().is_none());
        assert!(tokens[1].word.d8().is_none());
        assert_eq!(tokens[1].word.d9().unwrap().word().packed_bits(), 257);
    }

    #[test]
    fn same_numeric_payload_at_different_widths_never_collapses() {
        let tokens = parse_binary_source_words("1 01 001 0001").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.word.width()).collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
        assert!(tokens.iter().all(|token| token.word.packed_bits() == 1));
        assert_ne!(tokens[0].word, tokens[1].word);
        assert_ne!(tokens[1].word, tokens[2].word);
        assert_ne!(tokens[2].word, tokens[3].word);
    }

    #[test]
    fn d1_through_d9_bridges_are_exact_and_lossless() {
        for raw in 0..=1 {
            let source = BinarySourceWord::W1(Bit1::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d1().unwrap()), source);
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=3 {
            let source = BinarySourceWord::W2(Bit2::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d2().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=7 {
            let source = BinarySourceWord::W3(Bit3::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d3().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=15 {
            let source = BinarySourceWord::W4(Bit4::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d4().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=31 {
            let source = BinarySourceWord::W5(Bit5::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d5().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=63 {
            let source = BinarySourceWord::W6(Bit6::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d6().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=127 {
            let source = BinarySourceWord::W7(Bit7::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d7().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d8().is_none());
        }

        for raw in 0..=255 {
            let source = BinarySourceWord::W8(Bit8::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d8().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d9().is_none());
        }

        for raw in 0..=511 {
            let source = BinarySourceWord::W9(Bit9::new(raw).unwrap());
            assert_eq!(BinarySourceWord::from(source.d9().unwrap()), source);
            assert!(source.d1().is_none());
            assert!(source.d2().is_none());
            assert!(source.d3().is_none());
            assert!(source.d4().is_none());
            assert!(source.d5().is_none());
            assert!(source.d6().is_none());
            assert!(source.d7().is_none());
            assert!(source.d8().is_none());
        }
    }

    #[test]
    fn comments_and_whitespace_are_source_boundaries_only() {
        let tokens = parse_binary_source_words("10 ; open\n 001\t01 ; close\n").unwrap();
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.word.to_string())
                .collect::<Vec<_>>(),
            ["10", "001", "01"]
        );
    }

    #[test]
    fn human_names_and_mixed_tokens_fail_closed() {
        for source in ["людина", "10x", "(10)", "01,"] {
            let error = parse_binary_source_words(source).expect_err(source);
            assert_eq!(error.kind, ErrorKind::Parse);
            assert!(error.message.contains("only exact-width binary words"));
        }
    }

    #[test]
    fn word_wider_than_current_w9_carrier_fails_named() {
        let error = parse_binary_source_words("0000000000").expect_err("ten bits must fail");
        assert_eq!(error.kind, ErrorKind::Parse);
        assert!(error.message.contains("exceeds current 9-bit carrier"));
        assert_eq!(error.span, Span { start: 0, end: 10 });
    }

    #[test]
    fn empty_and_comment_only_source_are_valid_empty_programs() {
        assert!(parse_binary_source_words("").unwrap().is_empty());
        assert!(parse_binary_source_words(" ; comment only\n").unwrap().is_empty());
    }
}
