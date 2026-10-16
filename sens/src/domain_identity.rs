//! Exact width-qualified identity carrier for the W1→W9 ladder.
//!
//! Semantic ratification and callable Core-operation identity are deliberately
//! separate. Width never grants a semantic role by itself.
//!
//! - D1-D9 мають чинну семантичну authority згідно з Contract 11.8.
//! - D5 #3305, D6 #3393, D7 #3572, D8 #3960, D9 #4008.
//! - W1-W8 keep the one-byte fast path; D9 uses the exact non-truncating W9 carrier.
//! - Semantic residency і callable/mechanism admission лишаються окремими фактами;
//!   відсутній механізм може fail-closed навіть для ратифікованого resident.
//! - D7 is keyboard/v2 W7 data identity and never enters callable routing by width.
//! - D8 exact identity is current; a packed transport byte never establishes its domain.

use crate::{
    Bija3, BinarySourceWord, CoreD4, CoreD5, CoreD6, CoreD8, CoreD9, DomainD10, PredicateBit,
    Racana2, D7DataWord,
};
use std::fmt;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum DomainIdentity {
    D1(PredicateBit),
    D2(Racana2),
    D3(Bija3),
    D4(CoreD4),
    D5(CoreD5),
    D6(CoreD6),
    D7(D7DataWord),
    D8(CoreD8),
    D9(CoreD9),
    D10(DomainD10),
}

impl DomainIdentity {
    pub const fn width(self) -> usize {
        self.source_word().width()
    }

    /// Mechanical payload wide enough for every current exact domain.
    ///
    /// Width remains part of `DomainIdentity`; the numeric payload alone is
    /// never a semantic identity.
    pub const fn packed_bits(self) -> u16 {
        match self {
            Self::D1(value) => value.word().packed_bits() as u16,
            Self::D2(value) => value.word().packed_bits() as u16,
            Self::D3(value) => value.word().packed_bits() as u16,
            Self::D4(value) => value.word().packed_bits() as u16,
            Self::D5(value) => value.word().packed_bits() as u16,
            Self::D6(value) => value.word().packed_bits() as u16,
            Self::D7(value) => value.word().packed_bits() as u16,
            Self::D8(value) => value.word().packed_bits() as u16,
            Self::D9(value) => value.word().packed_bits(),
            Self::D10(value) => value.packed_bits(),
        }
    }

    /// Total lift from the canonical exact-width source layer.
    pub const fn from_source_word(source: BinarySourceWord) -> Self {
        match source {
            BinarySourceWord::W1(word) => Self::D1(PredicateBit::from_word(word)),
            BinarySourceWord::W2(word) => Self::D2(Racana2::from_word(word)),
            BinarySourceWord::W3(word) => Self::D3(Bija3::from_word(word)),
            BinarySourceWord::W4(word) => Self::D4(CoreD4::from_word(word)),
            BinarySourceWord::W5(word) => Self::D5(CoreD5::from_word(word)),
            BinarySourceWord::W6(word) => Self::D6(CoreD6::from_word(word)),
            BinarySourceWord::W7(word) => Self::D7(D7DataWord::from_word(word)),
            BinarySourceWord::W8(word) => Self::D8(CoreD8::from_word(word)),
            BinarySourceWord::W9(word) => Self::D9(CoreD9::from_word(word)),
            BinarySourceWord::W10(word) => Self::D9(DomainD10::from_word(word)),
            BinarySourceWord::W10(word) => Self::D10(word),
        }
    }

    pub const fn source_word(self) -> BinarySourceWord {
        match self {
            Self::D1(value) => BinarySourceWord::W1(value.word()),
            Self::D2(value) => BinarySourceWord::W2(value.word()),
            Self::D3(value) => BinarySourceWord::W3(value.word()),
            Self::D4(value) => BinarySourceWord::W4(value.word()),
            Self::D5(value) => BinarySourceWord::W5(value.word()),
            Self::D6(value) => BinarySourceWord::W6(value.word()),
            Self::D7(value) => BinarySourceWord::W7(value.word()),
            Self::D8(value) => BinarySourceWord::W8(value.word()),
            Self::D9(value) => BinarySourceWord::W9(value.word()),
            Self::D10(value) => BinarySourceWord::W10(value),
        }
    }

    /// Explicit callable/Core-operation projection.
    ///
    /// Це projection саме до callable/mechanism routing, а не реєстр
    /// семантичної residency.
    ///
    /// D3/D4/D5 мають admitted Core-operation projection. D6 є OWNER-RATIFIED
    /// 64/64 під #3393, але callable projection тут точковий: лише D6:001110
    /// ADD1 і D6:001111 SUB1 admitted під #3394 як LOWER_DOMAIN_COMPOSITION.
    /// Інші D6 residents лишаються fail-closed; residency не дорівнює mechanism.
    /// D8 так само OWNER-RATIFIED #3960.
    /// D9 OWNER-RATIFIED #4008 і матеріалізується як exact identity, але
    /// callability/mechanism admission лишається окремою і fail-closed.
    pub const fn core_operation(self) -> Option<CoreDomainIdentity> {
        match self {
            Self::D3(value) => Some(CoreDomainIdentity::D3(value)),
            Self::D4(value) => Some(CoreDomainIdentity::D4(value)),
            Self::D5(value) => Some(CoreDomainIdentity::D5(value)),
            Self::D6(value)
                if matches!(value.word().packed_bits(), 0b001110 | 0b001111) =>
            {
                Some(CoreDomainIdentity::D6(value))
            }
            Self::D8(value) if matches!(
                value.word().packed_bits(),
                0b11100000 | 0b11101010 | 0b11101101 | 0b11101110 | 0b11110100
            ) => {
                // Only the exact admitted D8 CHAR-CODE, READ, STRING-TO-SYMBOL,
                // SYMBOL-TO-STRING and WRITE-TO-STRING can project here. Residency alone
                // never grants callability to any other D8 coordinate.
                Some(CoreDomainIdentity::D8(value))
            }
            Self::D9(_) => None,
            Self::D10(_) => None,
            Self::D1(_) | Self::D2(_) | Self::D6(_) | Self::D7(_) | Self::D8(_) => None,
        }
    }
}

impl fmt::Debug for DomainIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.width();
        write!(
            formatter,
            "DomainIdentity<D{width}>({:0width$b})",
            self.packed_bits()
        )
    }
}

impl fmt::Display for DomainIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:0width$b}",
            self.packed_bits(),
            width = self.width()
        )
    }
}

/// Callable/Core-operation identity. This is intentionally not the universal
/// domain identity type.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum CoreDomainIdentity {
    D3(Bija3),
    D4(CoreD4),
    D5(CoreD5),
    D6(CoreD6),
    D8(CoreD8),
}

impl CoreDomainIdentity {
    pub const fn width(self) -> usize {
        self.source_word().width()
    }

    pub const fn packed_bits(self) -> u8 {
        match self {
            Self::D3(value) => value.word().packed_bits(),
            Self::D4(value) => value.word().packed_bits(),
            Self::D5(value) => value.word().packed_bits(),
            Self::D6(value) => value.word().packed_bits(),
            Self::D8(value) => value.word().packed_bits(),
        }
    }

    pub const fn from_source_word(source: BinarySourceWord) -> Option<Self> {
        DomainIdentity::from_source_word(source).core_operation()
    }

    pub const fn source_word(self) -> BinarySourceWord {
        match self {
            Self::D3(value) => BinarySourceWord::W3(value.word()),
            Self::D4(value) => BinarySourceWord::W4(value.word()),
            Self::D5(value) => BinarySourceWord::W5(value.word()),
            Self::D6(value) => BinarySourceWord::W6(value.word()),
            Self::D8(value) => BinarySourceWord::W8(value.word()),
        }
    }
}

impl fmt::Debug for CoreDomainIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self.width();
        write!(
            formatter,
            "CoreDomainIdentity<D{width}>({:0width$b})",
            self.packed_bits()
        )
    }
}

impl fmt::Display for CoreDomainIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:0width$b}",
            self.packed_bits(),
            width = self.width()
        )
    }
}

impl From<PredicateBit> for DomainIdentity {
    fn from(value: PredicateBit) -> Self { Self::D1(value) }
}
impl From<Racana2> for DomainIdentity {
    fn from(value: Racana2) -> Self { Self::D2(value) }
}
impl From<Bija3> for DomainIdentity {
    fn from(value: Bija3) -> Self { Self::D3(value) }
}
impl From<CoreD4> for DomainIdentity {
    fn from(value: CoreD4) -> Self { Self::D4(value) }
}
impl From<CoreD5> for DomainIdentity {
    fn from(value: CoreD5) -> Self { Self::D5(value) }
}
impl From<CoreD6> for DomainIdentity {
    fn from(value: CoreD6) -> Self { Self::D6(value) }
}
impl From<D7DataWord> for DomainIdentity {
    fn from(value: D7DataWord) -> Self { Self::D7(value) }
}
impl From<CoreD8> for DomainIdentity {
    fn from(value: CoreD8) -> Self { Self::D8(value) }
}
impl From<CoreD9> for DomainIdentity {
    fn from(value: CoreD9) -> Self { Self::D9(value) }
}

impl From<Bija3> for CoreDomainIdentity {
    fn from(value: Bija3) -> Self { Self::D3(value) }
}
impl From<CoreD4> for CoreDomainIdentity {
    fn from(value: CoreD4) -> Self { Self::D4(value) }
}
impl From<CoreD5> for CoreDomainIdentity {
    fn from(value: CoreD5) -> Self { Self::D5(value) }
}
impl From<CoreD6> for CoreDomainIdentity {
    fn from(value: CoreD6) -> Self { Self::D6(value) }
}
impl From<CoreD8> for CoreDomainIdentity {
    fn from(value: CoreD8) -> Self { Self::D8(value) }
}

impl From<CoreDomainIdentity> for DomainIdentity {
    fn from(value: CoreDomainIdentity) -> Self {
        match value {
            CoreDomainIdentity::D3(word) => Self::D3(word),
            CoreDomainIdentity::D4(word) => Self::D4(word),
            CoreDomainIdentity::D5(word) => Self::D5(word),
            CoreDomainIdentity::D6(word) => Self::D6(word),
            CoreDomainIdentity::D8(word) => Self::D8(word),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9};

    #[test]
    fn all_d1_d9_source_words_round_trip_through_domain_identity() {
        for raw in 0..=1 {
            let source = BinarySourceWord::W1(Bit1::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=3 {
            let source = BinarySourceWord::W2(Bit2::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=7 {
            let source = BinarySourceWord::W3(Bit3::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=15 {
            let source = BinarySourceWord::W4(Bit4::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=31 {
            let source = BinarySourceWord::W5(Bit5::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=63 {
            let source = BinarySourceWord::W6(Bit6::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=127 {
            let source = BinarySourceWord::W7(Bit7::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=255 {
            let source = BinarySourceWord::W8(Bit8::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
        for raw in 0..=511 {
            let source = BinarySourceWord::W9(Bit9::new(raw).unwrap());
            assert_eq!(DomainIdentity::from_source_word(source).source_word(), source);
        }
    }


    #[test]
    fn d8_symbol_to_string_exact_core_mechanism_excludes_one_bit_neighbor() {
        let admitted = DomainIdentity::from_source_word(
            BinarySourceWord::W8(Bit8::new(0b11101110).unwrap()),
        );
        assert!(matches!(
            admitted.core_operation(),
            Some(CoreDomainIdentity::D8(word))
                if word.word().packed_bits() == 0b11101110
        ), "ratified exact D8 SYMBOL-TO-STRING has its existing mechanism");

        let neighbor = DomainIdentity::from_source_word(
            BinarySourceWord::W8(Bit8::new(0b11101111).unwrap()),
        );
        assert!(neighbor.core_operation().is_none(),
            "single-bit adjacent D8 residence is not implicitly callable");
        let other_domain = DomainIdentity::from_source_word(
            BinarySourceWord::W9(Bit9::new(0b011101110).unwrap()),
        );
        assert!(other_domain.core_operation().is_none(),
            "identical payload in another domain grants no D8 callability");
    }

    #[test]
    fn w9_source_word_round_trips_without_byte_truncation() {
        for raw in [0u16, 1, 255, 256, 257, 511] {
            let source = BinarySourceWord::W9(Bit9::new(raw).unwrap());
            let identity = DomainIdentity::from_source_word(source);
            assert!(matches!(identity, DomainIdentity::D9(_)));
            assert_eq!(identity.width(), 9);
            assert_eq!(identity.packed_bits(), raw);
            assert_eq!(identity.source_word(), source);
        }
    }

    #[test]
    fn d9_does_not_collapse_to_equal_low_payload_in_d1_d8() {
        let d9 = DomainIdentity::from_source_word(
            BinarySourceWord::W9(Bit9::new(1).unwrap())
        );
        let d8 = DomainIdentity::from_source_word(
            BinarySourceWord::W8(Bit8::new(1).unwrap())
        );
        assert_eq!(d9.packed_bits(), d8.packed_bits());
        assert_ne!(d9, d8);

        let high = DomainIdentity::from_source_word(
            BinarySourceWord::W9(Bit9::new(0x101).unwrap())
        );
        assert_eq!(high.packed_bits(), 0x101);
        assert_ne!(high, d9);
    }

    #[test]
    fn equal_payloads_do_not_collapse_across_d1_d9() {
        let ids = [
            DomainIdentity::from_source_word(BinarySourceWord::W1(Bit1::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W2(Bit2::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W3(Bit3::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W4(Bit4::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W5(Bit5::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W6(Bit6::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W7(Bit7::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W8(Bit8::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W9(Bit9::new(1).unwrap())),
        ];
        for id in ids { assert_eq!(id.packed_bits(), 1); }
        for left in 0..ids.len() {
            for right in left + 1..ids.len() {
                assert_ne!(ids[left], ids[right]);
            }
        }
    }
}
