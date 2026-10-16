//! Механічна драбина D1–D10 без жодної Rust-семантики.
//!
//! Координата — рівно (ширина, біти). Значення, назви, закони,
//! допуск до виконання та оракули належать доменним джерелам SENS.
//! Це НЕ другий реєстр функцій і НЕ дозвіл на виконання D10.
//!
//! Аксіома бітності: ширина ключа = адреса домену. D*n* ніколи не
//! приймає ключ іншої довжини. Рівний packed payload у різних ширинах
//! — різні координати.
//!
//! D7: `knowledge/d7-keyboard-v2-ratified.json` — чинні 128/128
//! семибітних координат даних. Жодна координата не надає callability.

/// Exact 10-bit mechanical carrier, not a resident AST value or a callable
/// function. No implicit widening/narrowing to D9 or historical Function8.
/// The host stores its payload in u16, but only ten bits belong to SENS.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DomainD10 {
    bits: u16,
}

impl DomainD10 {
    pub const fn new(bits: u16) -> Option<Self> {
        if bits < (1u16 << 10) {
            Some(Self { bits })
        } else {
            None
        }
    }

    pub const fn packed_bits(self) -> u16 {
        self.bits
    }

    pub const fn coordinate(self) -> DomainCoordinate {
        DomainCoordinate {
            width: 10,
            bits: self.bits,
        }
    }

    pub const fn from_coordinate(coordinate: DomainCoordinate) -> Option<Self> {
        if coordinate.width != 10 {
            return None;
        }
        Self::new(coordinate.bits)
    }
}

/// A width-qualified, meaning-free code word in the D1–D10 ladder.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DomainCoordinate {
    width: u8,
    bits: u16,
}

impl DomainCoordinate {
    /// Fail closed for out-of-range rungs and truncated payloads.
    pub const fn new(width: u8, bits: u16) -> Option<Self> {
        if width == 0 || width > 10 || bits >= (1u16 << width) {
            return None;
        }
        Some(Self { width, bits })
    }

    /// Mechanically project any already-validated D1-D9 source identity.
    /// D7 is an exact W7 coordinate; this grants neither a sound role nor
    /// generic callability. D10 remains representable via `new` only.
    pub const fn from_domain_identity(identity: crate::DomainIdentity) -> Self {
        Self {
            width: identity.width() as u8,
            bits: identity.packed_bits(),
        }
    }

    pub const fn width(self) -> u8 {
        self.width
    }

    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Точний видимий запис однієї координати без домішування десяткових
    /// чисел, пробілів чи текстових назв. Ведучі нулі є частиною identity.
    /// Це механічний розбір, а НЕ доказ виконуваності.
    pub fn з_видимих_бітів(слово: &str) -> Option<Self> {
        let raw = слово.as_bytes();
        if !(1..=10).contains(&raw.len()) {
            return None;
        }
        let mut bits = 0u16;
        for &digit in raw {
            bits = (bits << 1)
                | match digit {
                    b'0' => 0,
                    b'1' => 1,
                    _ => return None,
                };
        }
        Self::new(raw.len() as u8, bits)
    }

    /// Точна source-проєкція збереженої координати; без округлення до байта.
    /// Canonical binary-source projection: emit each identity bit directly.
    /// No decimal formatting/parsing step can erase the width or leading zeroes.
    pub fn видимі_біти(self) -> String {
        let width = usize::from(self.width);
        let mut source = String::with_capacity(width);
        for offset in 0..width {
            let shift = width - 1 - offset;
            let bit = (self.bits >> shift) & 1;
            source.push(if bit == 0 { '0' } else { '1' });
        }
        source
    }

    /// Pure occupancy projection from owner-ratified domain sources.
    ///
    /// D3–D7 are a generated coordinate subset. D8/D9 have independent
    /// full-width owner certificates; Contract 11.9 separately ratifies
    /// D10 as one complete 1024/1024 domain, without runtime admission.
    /// D1/D2 structural/predicate roles stay unprojected.
    /// Some(true) proves occupancy, never a callable mechanism or human name.
    ///
    /// D7: all 128 current keyboard coordinates are resident, but remain
    /// typed text/data, never callable merely because of their width.
    pub fn owner_residency(self) -> Option<bool> {
        if self.width == 10 {
            // Whole-domain ratification; NOT physical T5 or callability.
            return Some(true);
        }
        if crate::domain_owner_generated::DOMAIN_OWNER_FULL_WIDTHS.contains(&self.width) {
            return Some(true);
        }
        if !(3..=7).contains(&self.width) {
            return None;
        }
        // Generated D3..D7 bitmap positions follow exact widths (index=width-3).
        // A mismatch must fail closed rather than reinterpret another domain.
        // This removes the per-lookup linear scan over all bitmap descriptors.
        let mask = crate::domain_owner_generated::DOMAIN_OWNER_BITMAPS
            .get(usize::from(self.width - 3))?;
        if mask.width != self.width {
            return None;
        }
        let slot = usize::from(self.bits / 64);
        Some(((mask.slots[slot] >> (self.bits % 64)) & 1) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::DomainCoordinate;

    #[test]
    fn all_rungs_are_width_qualified_without_guessing_meaning() {
        for width in 1..=10 {
            for bits in 0..(1u16 << width) {
                let coordinate = DomainCoordinate::new(width, bits).unwrap();
                assert_eq!(coordinate.width(), width);
                assert_eq!(coordinate.bits(), bits);
            }
            assert_eq!(DomainCoordinate::new(width, 1u16 << width), None);
        }
    }

    #[test]
    fn decimal_text_cannot_stand_in_for_a_width_qualified_identity() {
        let d3 = DomainCoordinate::new(3, 0b010).unwrap();
        let d4 = DomainCoordinate::new(4, 0b0010).unwrap();
        let d5 = DomainCoordinate::new(5, 0b00010).unwrap();

        assert_eq!(d3.видимі_біти(), "010");
        assert_eq!(d4.видимі_біти(), "0010");
        assert_eq!(d5.видимі_біти(), "00010");
        assert_ne!(d3, d4);
        assert_ne!(d4, d5);
        assert_eq!(DomainCoordinate::з_видимих_бітів("010"), Some(d3));
        assert_eq!(DomainCoordinate::з_видимих_бітів("0010"), Some(d4));
        assert_eq!(DomainCoordinate::з_видимих_бітів("00010"), Some(d5));
        assert_eq!(DomainCoordinate::з_видимих_бітів("2"), None);
        assert_eq!(DomainCoordinate::з_видимих_бітів("10"), DomainCoordinate::new(2, 0b10));
    }

    /// Механічна перевірка прямого lookup на межах бітмап.
    /// Очікування береться з незалежного списку координат генератора,
    /// а не з поточної бітмапи і НЕ є новою ратифікацією D4.
    #[test]
    fn прямий_бітмап_зберігає_точні_координати_та_межу_слота_d7() {
        for (ширина, біти) in [
            (3, 0b000),
            (3, 0b001),
            (4, 0b0100),
            (4, 0b0101),
            (4, 0b1001),
            (4, 0b1010),
            (7, 0b0111111), // останній біт першого u64
            (7, 0b1000000), // перший біт другого u64
            (7, 0b1111111),
        ] {
            let координата = DomainCoordinate::new(ширина, біти).unwrap();
            let є_у_проєкції = crate::domain_owner_generated::DOMAIN_OWNER_COORDINATES
                .iter()
                .any(|рядок| рядок.width == ширина && u16::from(рядок.bits) == біти);
            assert_eq!(
                координата.owner_residency(),
                Some(є_у_проєкції),
                "bitmap disagrees with generated owner-coordinate list at D{ширина}:{}",
                координата.видимі_біти(),
            );
        }
        for ширина in 1..=2 {
            assert_eq!(
                DomainCoordinate::new(ширина, 0).unwrap().owner_residency(),
                None,
                "D{ширина} cannot inherit a D3 bitmap",
            );
        }
    }

    /// Exhaustive W7 current keyboard occupancy is 128/128; no callable alias.
    #[test]
    fn d7_current_128_coordinates_are_resident_but_not_callable() {
        let mut admitted = 0usize;
        for bits in 0..128u16 {
            let coordinate = DomainCoordinate::new(7, bits).expect("exact W7");
            assert_eq!(coordinate.width(), 7);
            assert_eq!(coordinate.bits(), bits);
            let admission = coordinate.owner_residency().expect("D7 has owner rows");
            assert_eq!(
                admission,
                true,
                "every exact W7 keyboard coordinate is ratified data"
            );
            admitted += usize::from(admission);
            let typed = crate::DomainIdentity::from_source_word(
                crate::BinarySourceWord::W7(
                    crate::Bit7::new(bits as u8).expect("seven-bit word"),
                ),
            );
            assert_eq!(typed.width(), 7);
            assert_eq!(typed.packed_bits(), bits);
            assert!(typed.core_operation().is_none(), "D7 residency is not callability");
        }
        assert_eq!(admitted, 128);
        // Equal payload in another domain never inherits the D7 identity.
        assert_ne!(
            DomainCoordinate::new(6, 0b0100001).unwrap(),
            DomainCoordinate::new(7, 0b0100001).unwrap(),
            "same payload in D6 and D7 is not the same coordinate"
        );
        assert_eq!(
            DomainCoordinate::new(8, 0b0100001).unwrap().owner_residency(),
            Some(true),
            "D8 is separately owner-ratified, not inherited from D7"
        );
        assert_eq!(
            DomainCoordinate::new(10, 0b0100001).unwrap().owner_residency(),
            Some(true)
        );
    }

    #[test]
    fn d8_d9_all_coordinates_are_owner_residents_without_becoming_executable() {
        for (width, population) in [(8u8, 256u16), (9u8, 512u16)] {
            for bits in 0..population {
                let resident = DomainCoordinate::new(width, bits).unwrap();
                assert_eq!(resident.owner_residency(), Some(true));
                assert_ne!(resident, DomainCoordinate::new(10, bits).unwrap());
            }
        }
        for bits in [0, 1, 255, 511, 1023] {
            assert_eq!(
                DomainCoordinate::new(10, bits).unwrap().owner_residency(),
                Some(true),
                "whole D10 owner ratification is not an execution admission"
            );
        }
    }

    #[test]
    fn d10_stays_exactly_ten_bits_with_no_callable_projection() {
        assert_eq!(DomainCoordinate::new(10, 1023).unwrap().bits(), 1023);
        assert_eq!(DomainCoordinate::new(10, 1024), None);
        assert_ne!(DomainCoordinate::new(9, 511), DomainCoordinate::new(10, 511));
        for bits in 0..1024 {
            assert_eq!(DomainCoordinate::new(10, bits).unwrap().owner_residency(), Some(true));
        }
        assert_eq!(DomainCoordinate::new(0, 0), None);
        assert_eq!(DomainCoordinate::new(11, 0), None);
    }

    /// Вичерпний адресний тест 2046 координат D1–D10, а не твердження,
    /// що всі 2046 резидентів мають виконавчий механізм.
    /// Незалежний побітовий оракул ловить зсуви, втрату старшого біта,
    /// байтове вирівнювання між словами та втрату ведучих нулів.
    #[test]
    fn всі_2046_координат_зберігають_кожен_біт_через_джерело_і_пакування() {
        let mut пакувач = crate::BitPacker::new();
        let mut очікувані_біти = Vec::<u8>::new();
        let mut джерело = String::new();
        let mut адреси = Vec::new();
        let mut кількість = 0usize;

        for ширина in 1u8..=10 {
            for значення in 0u16..(1u16 << ширина) {
                let координата = DomainCoordinate::new(ширина, значення).unwrap();
                let слово = координата.видимі_біти();
                assert_eq!(слово.len(), usize::from(ширина));
                assert_eq!(
                    DomainCoordinate::з_видимих_бітів(&слово),
                    Some(координата),
                    "source D{ширина}:{слово}"
                );
                // Кожна одинична зміна біта має змінити рівно один
                // розряд координати, а не ширину або сусідній домен.
                for розряд in 0..usize::from(ширина) {
                    let mut мутація = слово.as_bytes().to_vec();
                    мутація[розряд] ^= 1; // ASCII '0' <-> '1'
                    let змінене = std::str::from_utf8(&мутація).unwrap();
                    let очікувана_мутація = DomainCoordinate::new(
                        ширина,
                        значення ^ (1u16 << (usize::from(ширина) - 1 - розряд)),
                    );
                    assert_eq!(
                        DomainCoordinate::з_видимих_бітів(змінене),
                        очікувана_мутація,
                        "одно-бітова мутація D{ширина}:{слово} at {розряд}"
                    );
                }
                let зміщення = пакувач.push_coordinate(координата);
                assert_eq!(зміщення, очікувані_біти.len());
                очікувані_біти.extend(слово.bytes().map(|bit| bit - b'0'));
                джерело.push_str(&слово);
                джерело.push(' ');
                адреси.push((зміщення, координата));
                кількість += 1;
            }
        }

        // Сума 2^n за n=1..10; сума n*2^n — точна вартість payload.
        assert_eq!(кількість, 2046);
        assert_eq!(очікувані_біти.len(), 18_434);
        let фактичний = пакувач.finish();
        assert_eq!(фактичний.bit_len(), 18_434);
        assert_eq!(фактичний.byte_len(), 2305);
        assert_eq!(фактичний.valid_bits_in_last_byte(), 2);

        let еталонні_байти: Vec<u8> = очікувані_біти
            .chunks(8)
            .map(|частина| {
                частина.iter().enumerate().fold(0u8, |acc, (індекс, bit)| {
                    acc | (bit << (7 - індекс))
                })
            })
            .collect();
        assert_eq!(фактичний.bytes(), еталонні_байти.as_slice());

        let розібрані = crate::parse_domain_ladder_source_words(&джерело)
            .expect("усі точні слова повинні розбиратися");
        assert_eq!(розібрані.len(), кількість);
        for ((зміщення, очікувана), токен) in адреси.into_iter().zip(розібрані) {
            assert_eq!(токен.coordinate, очікувана);
            assert_eq!(
                фактичний.read_coordinate(зміщення, очікувана.width()),
                Some(очікувана),
                "transport D{} at {зміщення}", очікувана.width()
            );
        }
        assert!(
            crate::PackedBitstream::from_parts(
                фактичний.bytes().to_vec(), фактичний.bit_len()
            ) == Some(фактичний),
            "мінімальне фізичне пакування повинно бути канонічним"
        );
    }

    #[test]
    fn некоректна_бітність_відхиляється_без_вгадування_домену() {
        for слово in ["", "2", "01x", "01 1", "10101010101", "０", "①", "0\\n"] {
            assert!(DomainCoordinate::з_видимих_бітів(слово).is_none(), "{слово:?}");
        }
        for ширина in 1u8..=10 {
            assert_eq!(
                DomainCoordinate::з_видимих_бітів(&"0".repeat(ширина as usize)),
                DomainCoordinate::new(ширина, 0)
            );
            assert_eq!(
                DomainCoordinate::з_видимих_бітів(&"1".repeat(ширина as usize)),
                DomainCoordinate::new(ширина, (1u16 << ширина) - 1)
            );
        }
        assert_ne!(
            DomainCoordinate::з_видимих_бітів("000000001"),
            DomainCoordinate::з_видимих_бітів("0000000001"),
            "D9 і D10 з однаковим payload не збігаються"
        );
    }

    #[test]
    fn every_d7_source_word_has_a_width_preserving_coordinate() {
        use crate::{BinarySourceWord, Bit7, DomainIdentity};

        for bits in 0u8..=127 {
            let word = BinarySourceWord::W7(Bit7::new(bits).unwrap());
            let identity = DomainIdentity::from_source_word(word);
            let coordinate = DomainCoordinate::from_domain_identity(identity);

            assert!(matches!(identity, DomainIdentity::D7(_)));
            assert_eq!(coordinate, DomainCoordinate::new(7, u16::from(bits)).unwrap());
            assert_ne!(coordinate, DomainCoordinate::new(8, u16::from(bits)).unwrap());
        }
    }
}
