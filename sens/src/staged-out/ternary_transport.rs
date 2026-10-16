//! ZIG-OWNED: physical bit-packing is the sterile Zig corridor's job (t5/f3c/transport.zig).
//! Rust is byte-addressed and must NOT pack: Zig hands it already-packed programs.
//! This module is retained only for the staged removal (#5617/#5609). Do not extend it.
//! Дослідний трійковий ТРАНСПОРТ двійкових слів SENS.
//!
//! Це не трійкова семантика: D1..D9 залишаються двійковими.
//! Транспортні цифри 0/1 відтворюють біт, 2 відділяє слова.
//! Упаковано п'ять тритів в один фізичний байт (3^5=243).
//! Фізичний EOF дає сам файл: окремий 22 не потрібний.
//! Завершальна неповна п'ятірка доповнюється лише тритами 2,
//! яких має бути 0..4; довший хвіст відхиляється.
//! Пробіл D7:W7 є даними, а фізичний трит 2 розділяє слова; D2:W2 не підмінюється.
//! Number D24+ ще не допускається до цього механічного носія.

use crate::{
    parse_binary_source_words, BinarySourceWord,
    Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TernaryTransportError {
    EmptyProgram,
    InvalidBinaryProjection,
    InvalidProgramSyntax,
    InvalidPhysicalByte,
    EmptyDomainWord,
    UnsupportedDomainWidth,
    InvalidTail,
    NoncanonicalEncoding,
    TransportTooLarge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TernaryTransportAccounting {
    pub word_count: usize,
    pub semantic_bits: usize,
    pub separating_trits: usize,
    pub encoded_trits: usize,
    pub tail_trits: usize,
    pub physical_bytes: usize,
    pub physical_bits: usize,
}

const TRITS_PER_BYTE: usize = 5;
const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;

/// Структурно зв'язані доменні слова віддані вже наявним типом.
/// Транспорт не змінює їхню двійкову ідентичність.
/// Один транспортний трит додається без проміжного масиву всіх тритів.
/// П'ять тритів утворюють число 0..242; `2` — тільки фізична межа.
#[inline]
fn append_t5_digit(
    out: &mut Vec<u8>,
    current: &mut u8,
    used: &mut usize,
    digit: u8,
) {
    debug_assert!(digit <= 2);
    *current = *current * 3 + digit;
    *used += 1;
    if *used == TRITS_PER_BYTE {
        out.push(*current);
        *current = 0;
        *used = 0;
    }
}

/// Один T5-алгоритм для вже валідованих доменних координат.
/// Значення читається з u16 без обрізання старшого біта D9/D10.
/// Трити 2 лише розділяють слова й заповнюють останню фізичну п'ятірку.
fn encode_ternary_fields(
    words: impl Iterator<Item = (usize, u16)> + ExactSizeIterator + Clone,
    max_width: usize,
) -> Result<Vec<u8>, TernaryTransportError> {
    let word_count = words.len();
    if word_count == 0 {
        return Err(TernaryTransportError::EmptyProgram);
    }

    let mut semantic_trits = 0usize;
    for (width, payload) in words.clone() {
        if width == 0 || width > max_width || width > 10 || payload >= (1u16 << width) {
            return Err(TernaryTransportError::UnsupportedDomainWidth);
        }
        semantic_trits = semantic_trits.checked_add(width)
            .ok_or(TernaryTransportError::TransportTooLarge)?;
    }
    let trit_count = semantic_trits.checked_add(word_count - 1)
        .ok_or(TernaryTransportError::TransportTooLarge)?;
    let byte_len = trit_count.checked_add(TRITS_PER_BYTE - 1)
        .ok_or(TernaryTransportError::TransportTooLarge)? / TRITS_PER_BYTE;
    if byte_len > MAX_FILE_BYTES {
        return Err(TernaryTransportError::TransportTooLarge);
    }

    let mut out = Vec::with_capacity(byte_len);
    let mut current = 0u8;
    let mut used = 0usize;
    for (position, (width, payload)) in words.enumerate() {
        if position != 0 {
            append_t5_digit(&mut out, &mut current, &mut used, 2);
        }
        for shift in (0..width).rev() {
            append_t5_digit(&mut out, &mut current, &mut used,
                ((payload >> shift) & 1) as u8);
        }
    }
    while used != 0 {
        append_t5_digit(&mut out, &mut current, &mut used, 2);
    }
    debug_assert_eq!(out.len(), byte_len);
    Ok(out)
}

/// Історичний D1–D9 інтерфейс: бітність кожного слова лишається незмінною.
pub fn encode_ternary_words(
    words: &[BinarySourceWord],
) -> Result<Vec<u8>, TernaryTransportError> {
    encode_ternary_fields(words.iter().map(|w| (w.width(), w.packed_bits())), 9)
}

/// Механічний T5-перенос D1–D10; координата сама по собі НЕ callable.
///
/// Не створює нового формату, не додає D10 до історичного Lisp-парсера
/// та не припускає рівності W8 і сучасної доменної тотожності D8.
#[cfg(test)]
pub fn encode_ternary_coordinates(
    coordinates: &[crate::domain_ladder::DomainCoordinate],
) -> Result<Vec<u8>, TernaryTransportError> {
    encode_ternary_fields(
        coordinates.iter().map(|c| (usize::from(c.width()), c.bits())),
        10,
    )
}

/// Lift a bit payload into the exact-width word carrier.
/// This is a mechanical width operation only; it assigns no resident meaning.
fn typed_binary_word(
    width: usize,
    payload: u16,
) -> Result<BinarySourceWord, TernaryTransportError> {
    Ok(match width {
        1 => BinarySourceWord::W1(
            Bit1::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        2 => BinarySourceWord::W2(
            Bit2::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        3 => BinarySourceWord::W3(
            Bit3::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        4 => BinarySourceWord::W4(
            Bit4::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        5 => BinarySourceWord::W5(
            Bit5::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        6 => BinarySourceWord::W6(
            Bit6::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        7 => BinarySourceWord::W7(
            Bit7::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        8 => BinarySourceWord::W8(
            Bit8::new(payload as u8).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        9 => BinarySourceWord::W9(
            Bit9::new(payload).ok_or(TernaryTransportError::UnsupportedDomainWidth)?,
        ),
        _ => return Err(TernaryTransportError::UnsupportedDomainWidth),
    })
}

/// Єдиний канонічний T5-декодер, параметризований межею допустимої
/// фізичної ширини та відповідним механічним носієм. Не допускає
/// довільного semantic routing чи підміни W8 і D8.
fn decode_ternary_fields<T>(
    data: &[u8],
    max_width: usize,
    from: fn(usize, u16) -> Result<T, TernaryTransportError>,
    project: fn(&T) -> (usize, u16),
) -> Result<Vec<T>, TernaryTransportError> {
    if data.is_empty() {
        return Err(TernaryTransportError::EmptyProgram);
    }
    if data.len() > MAX_FILE_BYTES {
        return Err(TernaryTransportError::TransportTooLarge);
    }
    if data.iter().any(|byte| *byte >= 243) {
        return Err(TernaryTransportError::InvalidPhysicalByte);
    }
    let mut tail = 0usize;
    'padding: for mut byte in data.iter().rev().copied() {
        for _ in 0..TRITS_PER_BYTE {
            if byte % 3 != 2 {
                break 'padding;
            }
            tail += 1;
            byte /= 3;
        }
    }
    if tail >= TRITS_PER_BYTE {
        return Err(TernaryTransportError::InvalidTail);
    }
    let useful_trits = data.len() * TRITS_PER_BYTE - tail;
    if useful_trits == 0 {
        return Err(TernaryTransportError::EmptyDomainWord);
    }

    let mut words = Vec::<T>::new();
    let mut current_value = 0u16;
    let mut current_width = 0usize;
    let mut consumed = 0usize;
    for mut byte in data.iter().copied() {
        if consumed == useful_trits {
            break;
        }
        let mut digits = [0u8; TRITS_PER_BYTE];
        for digit in digits.iter_mut().rev() {
            *digit = byte % 3;
            byte /= 3;
        }
        for digit in digits {
            if consumed == useful_trits {
                break;
            }
            consumed += 1;
            match digit {
                0 | 1 => {
                    current_width += 1;
                    // Перевірити ширину ДО зсуву, щоб не втратити старші біти.
                    if current_width > max_width || current_width > 10 {
                        return Err(TernaryTransportError::UnsupportedDomainWidth);
                    }
                    current_value = (current_value << 1) | u16::from(digit);
                }
                2 => {
                    if current_width == 0 {
                        return Err(TernaryTransportError::EmptyDomainWord);
                    }
                    words.push(from(current_width, current_value)?);
                    current_value = 0;
                    current_width = 0;
                }
                _ => unreachable!(),
            }
        }
    }
    if current_width == 0 {
        return Err(TernaryTransportError::EmptyDomainWord);
    }
    words.push(from(current_width, current_value)?);
    // Канонічність також перевіряє рівно 0–4 заключних трита 2.
    if encode_ternary_fields(words.iter().map(project), max_width)? != data {
        return Err(TernaryTransportError::NoncanonicalEncoding);
    }
    Ok(words)
}

/// Зберегти чинний вузький D1–D9 API без зміни жодного wire-byte.
pub fn decode_ternary_words(
    data: &[u8],
) -> Result<Vec<BinarySourceWord>, TernaryTransportError> {
    decode_ternary_fields(data, 9, typed_binary_word,
        |w| (w.width(), w.packed_bits()))
}

/// Прочитати D1–D10 фізичні слова як механічні координати.
///
/// D10 тут не є автоматично дозволеним викликом функції.
#[cfg(test)]
pub fn decode_ternary_coordinates(
    data: &[u8],
) -> Result<Vec<crate::domain_ladder::DomainCoordinate>, TernaryTransportError> {
    decode_ternary_fields(data, 10,
        |width, payload| {
            let width = u8::try_from(width)
                .map_err(|_| TernaryTransportError::UnsupportedDomainWidth)?;
            crate::domain_ladder::DomainCoordinate::new(width, payload)
                .ok_or(TernaryTransportError::UnsupportedDomainWidth)
        },
        |c| (usize::from(c.width()), c.bits()),
    )
}

/// Вертикальний вигляд — тільки для людини, newline не є
/// тритом і не записується в сам .sens файл.
pub fn render_ternary_words_vertical(words: &[BinarySourceWord]) -> String {
    words.iter().map(|word| format!("{word}\n")).collect()
}

/// Звичайне людське представлення фізичного .sens: транспортний трит 2
/// перетворюється тільки на пробіл МІЖ словами. Кінцевий
/// padding ніколи не потрапляє у відкритий для людини текст.
/// Це НЕ фізичний вміст файла: не записувати цей рядок у .sens.
pub fn render_ternary_words_spaced(words: &[BinarySourceWord]) -> String {
    // Лише механічний перегляд: біти слова і його початкова ширина.
    // Не створюємо тимчасовий Vec<String> чи String для кожного слова.
    let capacity: usize = words.iter().map(|word| word.width()).sum::<usize>()
        + words.len().saturating_sub(1);
    let mut visible = String::with_capacity(capacity);
    for (index, word) in words.iter().enumerate() {
        if index != 0 {
            visible.push(' ');
        }
        let bits = word.packed_bits();
        for shift in (0..word.width()).rev() {
            visible.push(if ((bits >> shift) & 1) == 0 { '0' } else { '1' });
        }
    }
    debug_assert_eq!(visible.len(), capacity);
    visible
}

/// Відкрити .sens у вигляді вихідних двійкових слів, розділених
/// одним ASCII пробілом. Декодер спершу перевіряє канонічний T5
/// та синтаксичні закони D2. Не відображати жодної транспортної '2'.
pub fn open_ternary_program(data: &[u8]) -> Result<String, TernaryTransportError> {
    let words = decode_ternary_program(data)?;
    Ok(render_ternary_words_spaced(&words))
}

/// Parse already-decoded domain words via the *same* packed-bit D2 reader
/// used by the canonical physical binary payload path. Never render text.
pub(crate) fn parse_t5_domain_words(
    words: &[BinarySourceWord],
) -> Result<Vec<crate::Expr>, crate::LanguageError> {
    crate::parse_canonical_word_sequence(words)
}

/// Поки що адаптер приймає видиму точну двійкову проєкцію D1..D9,
/// а не довільні історичні чи українські Lisp-імена.
pub fn encode_binary_projection_ternary(
    projection: &str,
) -> Result<Vec<u8>, TernaryTransportError> {
    let words = parse_binary_source_words(projection)
        .map_err(|_| TernaryTransportError::InvalidBinaryProjection)?;
    if words.is_empty() {
        return Err(TernaryTransportError::EmptyProgram);
    }
    let exact_words = words.iter().map(|token| token.word).collect::<Vec<_>>();
    parse_t5_domain_words(&exact_words)
        .map_err(|_| TernaryTransportError::InvalidProgramSyntax)?;
    encode_ternary_words(&exact_words)
}

/// Формально валідує повноту D2-структури через наявний
/// рідер SENS, а не вигадує ще один парсер у transport.
pub fn decode_ternary_program(
    data: &[u8],
) -> Result<Vec<BinarySourceWord>, TernaryTransportError> {
    let words = decode_ternary_words(data)?;
    // The D2 grammar consumes typed binary words, not a rendered text surface.
    parse_t5_domain_words(&words)
        .map_err(|_| TernaryTransportError::InvalidProgramSyntax)?;
    Ok(words)
}

/// Вартість носія, включно з фізичним заповненням, але БЕЗ EOS.
/// Трити і біти — різні одиниці; физичні байти враховано окремо.
pub fn ternary_transport_accounting(
    words: &[BinarySourceWord],
) -> Result<TernaryTransportAccounting, TernaryTransportError> {
    let data = encode_ternary_words(words)?;
    let semantic_bits = words.iter().map(|word| word.width()).sum::<usize>();
    let separating_trits = words.len() - 1;
    let encoded_trits = semantic_bits + separating_trits;
    let tail_trits = data.len() * TRITS_PER_BYTE - encoded_trits;
    Ok(TernaryTransportAccounting {
        word_count: words.len(),
        semantic_bits,
        separating_trits,
        encoded_trits,
        tail_trits,
        physical_bytes: data.len(),
        physical_bits: data.len() * 8,
    })
}

/// Ukrainian keyboard D7 record on the *existing* canonical T5 physical wire.
/// No additional tag, no semantic remapping, and never a D2 structural token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum D7T5Error {
    Transport(TernaryTransportError),
    EmptyText,
    UnassignedKeyboardGlyph { byte_offset: usize },
    NonD7Word,
    UnrenderableD7Cell,
    AmbiguousText,
}

impl From<TernaryTransportError> for D7T5Error {
    fn from(source: TernaryTransportError) -> Self {
        Self::Transport(source)
    }
}

fn keyboard_d7_words(source: &str) -> Result<Vec<BinarySourceWord>, D7T5Error> {
    if source.is_empty() {
        return Err(D7T5Error::EmptyText);
    }
    // D7 v2 assigns each Ukrainian keyboard character one exact W7 cell.
    // The old D7 uppercase modifier must not add a phantom word.
    source.char_indices().map(|(offset, ch)| {
        // Four stack bytes fit every Unicode scalar: no heap String per D7 glyph.
        // The existing canonical D7 table still determines the exact W7 cell.
        let mut utf8 = [0u8; 4];
        let glyph = ch.encode_utf8(&mut utf8);
        let cell = crate::semantic_registry::d7_uk_keyboard_cell(glyph)
            .ok_or(D7T5Error::UnassignedKeyboardGlyph { byte_offset: offset })?;
        Ok(BinarySourceWord::W7(Bit7::new(cell).expect("D7 table is exactly W7")))
    }).collect()
}

/// Map owner-ratified Ukrainian keyboard glyphs to exact W7 data words.
/// No physical T5 encoding takes place here; external transports (including
/// Zig ABI v2) may consume this checked, width-qualified SENS projection.
pub fn encode_d7_uk_words(source: &str) -> Result<Vec<BinarySourceWord>, D7T5Error> {
    let words = keyboard_d7_words(source)?;
    // The same owner D7 table already defines the reverse projection.
    // Check each exact W7 coordinate before packing; decoding the complete
    // packed file here would add a redundant T5 pass and allocate a String.
    for (character, word) in source.chars().zip(words.iter()) {
        let BinarySourceWord::W7(cell) = word else {
            return Err(D7T5Error::NonD7Word);
        };
        let canonical = crate::semantic_registry::d7_uk_keyboard_glyph(cell.packed_bits())
            .ok_or(D7T5Error::UnrenderableD7Cell)?;
        let mut utf8 = [0u8; 4];
        let input: &str = character.encode_utf8(&mut utf8);
        if canonical != input {
            return Err(D7T5Error::AmbiguousText);
        }
    }
    Ok(words)
}

/// Physical Rust T5 reference remains supported independently of Zig.
pub fn encode_d7_uk_t5(source: &str) -> Result<Vec<u8>, D7T5Error> {
    Ok(encode_ternary_words(&encode_d7_uk_words(source)?)?)
}

/// Render only ratified W7 Ukrainian keyboard words, without touching T5.
/// D2 words (00/01/10/11) never become D7 text or punctuation.
pub fn decode_d7_uk_words(words: &[BinarySourceWord]) -> Result<String, D7T5Error> {
    let mut text = String::new();
    for word in words {
        let BinarySourceWord::W7(cell) = word else {
            return Err(D7T5Error::NonD7Word);
        };
        let glyph = crate::semantic_registry::d7_uk_keyboard_glyph(cell.packed_bits())
            .ok_or(D7T5Error::UnrenderableD7Cell)?;
        text.push_str(glyph);
    }
    if keyboard_d7_words(&text)?.as_slice() != words {
        return Err(D7T5Error::AmbiguousText);
    }
    Ok(text)
}

/// Physical Rust T5 reference remains supported independently of Zig.
pub fn decode_d7_uk_t5(packed: &[u8]) -> Result<String, D7T5Error> {
    decode_d7_uk_words(&decode_ternary_words(packed)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(source: &str) -> Vec<BinarySourceWord> {
        parse_binary_source_words(source).unwrap()
            .into_iter().map(|token| token.word).collect()
    }

    #[test]
    fn точковий_т5_кожний_біт_d10_за_d3_d4_без_обрізання() {
        use crate::domain_ladder::DomainCoordinate;
        let d3 = DomainCoordinate::new(3, 0b101).unwrap();
        let d4 = DomainCoordinate::new(4, 0b0010).unwrap();
        // 3 + 4 + 10 = 17 семантичних бітів; два транспортні
        // роздільники = 19 тритів, рівно чотири фізичні байти.
        // Залишок: один заключний трит 2. Жодного округлення між словами.
        for payload in 0u16..1024 {
            let d10 = DomainCoordinate::new(10, payload).unwrap();
            let source = [d3, d4, d10];
            let physical = encode_ternary_coordinates(&source).unwrap();
            assert_eq!(physical.len(), 4, "D10={payload}");
            assert_eq!(decode_ternary_coordinates(&physical).unwrap(), source, "D10={payload}");
            assert_eq!(decode_ternary_words(&physical),
                Err(TernaryTransportError::UnsupportedDomainWidth),
                "старий D1–D9 читач має fail-closed при D10={payload}");
            // Підміна єдиного транспортного padding-трита значущим бітом
            // не може непомітно змінити десятирозрядне слово.
            let mut altered = physical;
            *altered.last_mut().unwrap() -= 1;
            assert!(decode_ternary_coordinates(&altered).is_err(), "tail D10={payload}");
        }
        assert!(DomainCoordinate::new(10, 1024).is_none());
        assert!(DomainCoordinate::new(11, 0).is_none());
    }

    #[test]
    fn точковий_т5_кожен_біт_впливає_лише_на_свій_домен() {
        use crate::domain_ladder::DomainCoordinate;

        let d3 = DomainCoordinate::new(3, 0b101).unwrap();
        let d4 = DomainCoordinate::new(4, 0b0010).unwrap();
        let mut flips = 0usize;
        for payload in 0u16..1024 {
            let original = [
                d3,
                d4,
                DomainCoordinate::new(10, payload).unwrap(),
            ];
            let canonical = encode_ternary_coordinates(&original).unwrap();
            // Трит 2 стоїть тільки на позиціях 3, 8 і 19 (padding).
            for position in 0..19 {
                if position == 3 || position == 8 {
                    continue;
                }
                let byte_index = position / TRITS_PER_BYTE;
                let significance = 3u8.pow((TRITS_PER_BYTE - 1 - position % TRITS_PER_BYTE) as u32);
                let digit = (canonical[byte_index] / significance) % 3;
                assert!(digit <= 1, "semantic trit became separator");
                let mut altered = canonical.clone();
                if digit == 0 {
                    altered[byte_index] += significance;
                } else {
                    altered[byte_index] -= significance;
                }
                let decoded = decode_ternary_coordinates(&altered)
                    .expect("переворот значущого трита не є пошкодженням кадру");
                let (domain, local_bit) = match position {
                    0..=2 => (0, position),
                    4..=7 => (1, position - 4),
                    9..=18 => (2, position - 9),
                    _ => unreachable!(),
                };
                for (index, observed) in decoded.iter().enumerate() {
                    let expected_bits = if index == domain {
                        original[index].bits() ^ (1u16 << (usize::from(original[index].width()) - 1 - local_bit))
                    } else {
                        original[index].bits()
                    };
                    assert_eq!(observed.width(), original[index].width(),
                        "D10={payload} physical_trit={position}");
                    assert_eq!(observed.bits(), expected_bits,
                        "D10={payload} physical_trit={position} domain={index}");
                }
                assert_eq!(encode_ternary_coordinates(&decoded).unwrap(), altered);
                flips += 1;
            }
        }
        // (3 + 4 + 10) * 1024: жоден біт не перевіряється лише опосередковано.
        assert_eq!(flips, 17 * 1024);
    }

    #[test]
    fn точковий_т5_девять_старих_доменів_байт_у_байт_сумісні() {
        use crate::domain_ladder::DomainCoordinate;
        for width in 1usize..=9 {
            for payload in 0u16..(1u16 << width) {
                let legacy = typed_binary_word(width, payload).unwrap();
                let current = DomainCoordinate::new(width as u8, payload).unwrap();
                let old_wire = encode_ternary_words(&[legacy]).unwrap();
                let new_wire = encode_ternary_coordinates(&[current]).unwrap();
                assert_eq!(old_wire, new_wire, "D{width} payload={payload}");
                assert_eq!(decode_ternary_words(&old_wire).unwrap(), vec![legacy]);
                assert_eq!(decode_ternary_coordinates(&new_wire).unwrap(), vec![current]);
            }
        }
        let nine = DomainCoordinate::new(9, 1).unwrap();
        let ten = DomainCoordinate::new(10, 1).unwrap();
        assert_ne!(nine, ten);
        assert_ne!(encode_ternary_coordinates(&[nine]).unwrap(),
                   encode_ternary_coordinates(&[ten]).unwrap());
    }

    #[test]
    fn five_binary_words_survive_without_file_whitespace() {
        let source = "10 001 00 000 01";
        let original = words(source);
        let binary = encode_binary_projection_ternary(source).unwrap();
        assert_eq!(binary.len(), 4);
        assert_eq!(decode_ternary_program(&binary).unwrap(), original);
        assert_eq!(render_ternary_words_vertical(&original),
                   "10\n001\n00\n000\n01\n");
        let measure = ternary_transport_accounting(&original).unwrap();
        assert_eq!(measure.semantic_bits, 12);
        assert_eq!(measure.separating_trits, 4);
        assert_eq!(measure.encoded_trits, 16);
        assert_eq!(measure.tail_trits, 4);
        assert_eq!(measure.physical_bits, 32);
    }

    #[test]
    fn opening_binary_sens_shows_only_spaces_not_transport_twos() {
        let original = "10 001 00 000 01";
        let binary = encode_binary_projection_ternary(original).unwrap();
        assert_eq!(binary, [0x63, 0x89, 0x06, 0xa1]);
        let visible = open_ternary_program(&binary).unwrap();
        assert_eq!(visible, original);
        assert_eq!(visible.bytes().filter(|c| *c == b' ').count(), 4);
        assert!(!visible.contains('2'));
        assert!(!visible.contains('\n'));
        // View is a projection, physical SENS remains nontext packed bytes.
        assert_ne!(visible.as_bytes(), binary);
        assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), binary);
    }

    #[test]
    fn opening_never_renders_padding_or_d7_space_as_a_separator_token() {
        let source = "10 001 00 1100000 01";
        let encoded = encode_binary_projection_ternary(source).unwrap();
        assert_eq!(open_ternary_program(&encoded).unwrap(), source);
        assert_eq!(render_ternary_words_spaced(&[]), "");
        let changed = [243u8]; // impossible packed 5-trit value
        assert_eq!(
            open_ternary_program(&changed),
            Err(TernaryTransportError::InvalidPhysicalByte)
        );
    }

    #[test]
    fn identical_naked_payload_no_longer_loses_domain_boundaries() {
        let a = words("0 00");
        let b = words("000");
        let encoded_a = encode_ternary_words(&a).unwrap();
        let encoded_b = encode_ternary_words(&b).unwrap();
        assert_ne!(encoded_a, encoded_b);
        assert_eq!(decode_ternary_words(&encoded_a).unwrap(), a);
        assert_eq!(decode_ternary_words(&encoded_b).unwrap(), b);
    }

    #[test]
    fn exhaustive_single_word_roundtrip_d1_to_d9() {
        for width in 1..=9 {
            for value in 0..(1usize << width) {
                let sample = format!("{value:0width$b}");
                let original = words(&sample);
                let physical = encode_ternary_words(&original).unwrap();
                assert_eq!(decode_ternary_words(&physical).unwrap(), original);
            }
        }
    }

    #[test]
    fn d7_sign_space_is_payload_not_transport_separator() {
        let original = words("1100000 101 1100000");
        let physical = encode_ternary_words(&original).unwrap();
        assert_eq!(decode_ternary_words(&physical).unwrap(), original);
    }

    #[test]
    fn file_eof_and_padding_are_canonical_without_22() {
        let original = words("1");
        let physical = encode_ternary_words(&original).unwrap();
        assert_eq!(decode_ternary_words(&physical).unwrap(), original);
        // Голий single-word теж має кінець: EOF файла, не закривальна дужка.
        assert_eq!(physical.len(), 1);
        let mut extra = physical.clone();
        extra.push(242); // п'ять зайвих тритів 2 — не канонічний padding
        assert_eq!(decode_ternary_words(&extra),
                   Err(TernaryTransportError::InvalidTail));
        assert_eq!(decode_ternary_words(&[243]),
                   Err(TernaryTransportError::InvalidPhysicalByte));
        assert_eq!(decode_ternary_words(&[242]),
                   Err(TernaryTransportError::InvalidTail));
        // Старий EOF=22 не дозволено приймати як новий canonical wire,
        // якщо він створює зайвий фізичний байт.
        let closed = words("10 01");
        assert_eq!(encode_ternary_words(&closed).unwrap(), [0x64]);
        assert_eq!(decode_ternary_words(&[0x64, 0xf2]),
                   Err(TernaryTransportError::InvalidTail));
    }

    #[test]
    fn bracket_close_is_not_required_by_physical_eof() {
        for source in ["000", "1", "10 01", "10 001 01",
                       "10 001 01 10 000 01"] {
            let original = words(source);
            let encoded = encode_ternary_words(&original).unwrap();
            assert_eq!(decode_ternary_words(&encoded).unwrap(), original, "{source}");
            assert_eq!(decode_ternary_program(&encoded).unwrap(), original, "{source}");
        }
        // Крайній випадок: код у 5 тритів, padding відсутній.
        let single_exact = words("00000");
        let physical = encode_ternary_words(&single_exact).unwrap();
        assert_eq!(physical.len(), 1);
        assert_eq!(decode_ternary_words(&physical).unwrap(), single_exact);
    }

    #[test]
    fn no_eos_saves_byte_for_short_parenthesized_forms() {
        for (source, expected_bytes) in [
            ("10 01", 1usize),
            ("10 001 01", 2usize),
        ] {
            let original = words(source);
            let physical = encode_ternary_words(&original).unwrap();
            assert_eq!(physical.len(), expected_bytes);
            assert_eq!(decode_ternary_program(&physical).unwrap(), original);
        }
    }

    #[test]
    fn reject_unclosed_grammar_and_unratified_word_width() {
        assert_eq!(encode_binary_projection_ternary("10 001"),
                   Err(TernaryTransportError::InvalidProgramSyntax));
        assert_eq!(encode_binary_projection_ternary("0000000000"),
                   Err(TernaryTransportError::InvalidBinaryProjection));
        assert_eq!(encode_binary_projection_ternary(""),
                   Err(TernaryTransportError::EmptyProgram));
    }

    #[test]
    fn mixed_width_words_and_nested_form_roundtrip() {
        let source = "10 001 00 10 000 01 01";
        let binary = encode_binary_projection_ternary(source).unwrap();
        assert_eq!(decode_ternary_program(&binary).unwrap(), words(source));
        for w in 1..=9 {
            let short = "1".repeat(w);
            let input = words(&format!("10 {short} 01"));
            let bytes = encode_ternary_words(&input).unwrap();
            assert_eq!(decode_ternary_words(&bytes).unwrap(), input);
        }
    }

    #[test]
    fn every_possible_terminal_padding_length_is_canonical() {
        let mut seen = [false; 5];
        // A single exact-width word of each length 1..9 exercises all
        // possible byte remainders. No D2 CLOSE and no EOS are needed.
        for width in 1usize..=9 {
            let source = "1".repeat(width);
            let original = words(&source);
            let physical = encode_ternary_words(&original).unwrap();
            let account = ternary_transport_accounting(&original).unwrap();
            assert_eq!(account.encoded_trits, width);
            assert!(account.tail_trits < 5);
            seen[account.tail_trits] = true;
            assert_eq!(decode_ternary_words(&physical).unwrap(), original);
        }
        assert!(seen.iter().all(|value| *value));
    }

    #[test]
    fn explicit_double_separator_is_not_an_eof_marker() {
        let original = words("10 01");
        let canonical = encode_ternary_words(&original).unwrap();
        assert_eq!(canonical, [0x64]); // 10201, exactly 5 trits.
        // A second block of pure padding cannot be mistaken for EOF.
        assert_eq!(decode_ternary_words(&[0x64, 0xf2]),
                   Err(TernaryTransportError::InvalidTail));
        // T5 separator between two words continues to preserve exact widths.
        let a = words("0 00");
        let b = words("000");
        assert_ne!(encode_ternary_words(&a).unwrap(), encode_ternary_words(&b).unwrap());
    }

    #[test]
    fn trailing_padding_must_not_become_a_program_word() {
        let original = words("10 001 00 000 01");
        let binary = encode_ternary_words(&original).unwrap();
        let mut mutated = binary.clone();
        *mutated.last_mut().unwrap() -= 1;
        assert!(decode_ternary_words(&mutated).is_err());
    }

    /// Mechanic-only corpus witness. It does NOT parse or execute a SENS program.
    /// Every git-tracked .sens must preserve its exact D1..D10 coordinate widths,
    /// payload bits, order, and physical bytes under the existing Rust T5 codec.
    #[test]
    fn git_tracked_physical_t5_corpus_roundtrips_exact_coordinates() {
        use std::path::Path;
        use std::process::Command;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let files = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["ls-files", "-z", "--", "*.sens"])
            .output()
            .expect("T5 corpus: git executable required");
        assert!(
            files.status.success(),
            "T5 corpus: git ls-files failed: {}",
            String::from_utf8_lossy(&files.stderr)
        );

        let mut file_count = 0usize;
        let mut d10_coordinates = 0usize;
        let mut saw_d10_bridge = false;
        for raw in files.stdout.split(|byte| *byte == 0).filter(|raw| !raw.is_empty()) {
            let relative_path = std::str::from_utf8(raw)
                .expect("T5 corpus: non-UTF-8 tracked filename (fail closed)");
            let physical = std::fs::read(root.join(relative_path))
                .unwrap_or_else(|err| panic!("T5 corpus: read {relative_path}: {err}"));
            let coordinates = decode_ternary_coordinates(&physical)
                .unwrap_or_else(|err| panic!("T5 corpus: decode {relative_path}: {err:?}"));
            let replay = encode_ternary_coordinates(&coordinates)
                .unwrap_or_else(|err| panic!("T5 corpus: encode {relative_path}: {err:?}"));
            if physical != replay {
                let first_diff = physical.iter().zip(&replay)
                    .position(|(a, b)| a != b)
                    .unwrap_or(physical.len().min(replay.len()));
                panic!(
                    "T5 corpus byte mismatch: file={relative_path} layer=transport \
                     expected_len={} actual_len={} first_diff={} \
                     expected_byte={:?} actual_byte={:?} \
                     reproduce='cargo test --locked -p sens --lib \
                     ternary_transport::tests::git_tracked_physical_t5_corpus_roundtrips_exact_coordinates -- --exact --nocapture'",
                    physical.len(), replay.len(), first_diff,
                    physical.get(first_diff), replay.get(first_diff)
                );
            }
            if relative_path == "lib/bridge/prolog-to-datalog.sens" {
                saw_d10_bridge = true;
                assert!(
                    coordinates.iter().any(|coordinate| coordinate.width() == 10),
                    "T5 corpus: bridge must preserve its explicit D10 coordinate"
                );
                assert_eq!(
                    decode_ternary_words(&physical),
                    Err(TernaryTransportError::UnsupportedDomainWidth),
                    "D10 coordinates do not grant D1..D9 execution transport admission"
                );
            }
            d10_coordinates += coordinates
                .iter()
                .filter(|coordinate| coordinate.width() == 10)
                .count();
            file_count += 1;
        }
        assert!(file_count > 0, "T5 corpus: no git-tracked .sens files");
        assert!(saw_d10_bridge, "T5 corpus: expected D10 bridge missing from tracked files");
        eprintln!(
            "RUST_T5_CORPUS_COORDINATES_PASS layer=transport files={file_count} d10_words={d10_coordinates} semantic_authority=SENS"
        );
    }
}
