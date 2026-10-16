//! Envelope E3: domain-aware self-contained carrier for Core D1-D10 and explicit D24+.
//!
//! Historical wire/research name: S3/v3.  This is an Envelope, not Frame3/F3;
//! the old filename/API is retained only for compatibility with evidence/tests.
//!
//! This is not the canonical .sens format, not a semantic oracle, and does not
//! authorize any arithmetic/coercion. Numeric identity is (ladder index, full
//! exact-width bytes); equal magnitudes in D24 and D48 are DIFFERENT words.
//!
//! S3 03 | canonical ULEB word-count | canonical byte-DP blocks | CRC32.
//! RAW (0): count, per-word descriptors (1..9 for Core, 10 for exact D10, 0x80 + ULEB
//! numeric ladder index), then tightly packed bits in descriptor order.
//! REPEAT (1): count, descriptor, one aligned payload.
//! NUMERIC_RUN (2): count, shared numeric ladder index, aligned payloads.
//! GRAMMAR (3): count, exact inner bit length, fixed-width D2 rank;
//! bounded to the independently proven 20-inner-bit grammar implementation.
//!
//! This module does not reinterpret 24-bit payload as a D24 function or
//! invent numeric literals, resident occupancy, signedness, or widening.

use crate::{parse_canonical_word_sequence, BinarySourceWord, Bit1};
use crate::domain_words::DomainD10;
use crate::frame3_grammar::{count_inner, rank_binary_source_words, unrank_binary_source_words, ExactWord, MAX_INNER_BITS};

const MAGIC: [u8; 3] = [b'S', b'3', 3];
const RAW: u8 = 0;
const REPEAT: u8 = 1;
const NUMERIC_RUN: u8 = 2;
const GRAMMAR: u8 = 3;
const NUMERIC_DESCRIPTOR: u8 = 0x80;
const MAX_WORDS: usize = 4096;
const MAX_BLOCK_WORDS: usize = 128;
const MAX_BYTES: usize = 1 << 20;
const MAX_NUMERIC_LEVEL: u8 = 18; // 786432-byte D6291456 fits MAX_BYTES; resource bound, not a domain-law limit

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Word {
    Core(BinarySourceWord),
    /// Exact D10 coordinate. A u16 host slot carries only ten semantic bits.
    D10(DomainD10),
    Numeric { level: u8, bytes: Vec<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    Malformed,
    Truncated,
    BadVersion,
    NonCanonical,
    Integrity,
    InvalidD2,
}

impl Word {
    /// `level=0` means D24; `1` D48; `2` D96, ... . Explicitly supplied
    /// domain identity is retained even if the magnitude has leading zeros.
    pub fn numeric(level: u8, bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.len() != numeric_bytes(level)? { return Err(Error::Malformed); }
        Ok(Self::Numeric { level, bytes })
    }

    fn width_bits(&self) -> usize {
        match self {
            Self::Core(w) => w.width(),
            Self::D10(_) => 10,
            Self::Numeric { bytes, .. } => bytes.len() * 8,
        }
    }

    fn descriptor_cost(&self) -> usize {
        match self {
            Self::Core(_) | Self::D10(_) => 1,
            Self::Numeric { level, .. } => 1 + uleb_len(*level as usize),
        }
    }
}

fn numeric_bytes(level: u8) -> Result<usize, Error> {
    if level > MAX_NUMERIC_LEVEL { return Err(Error::TooLarge); }
    let bytes = 3usize.checked_shl(u32::from(level)).ok_or(Error::TooLarge)?;
    if bytes > MAX_BYTES { return Err(Error::TooLarge); }
    Ok(bytes)
}

fn uleb_len(mut x: usize) -> usize {
    let mut n = 1;
    while x >= 128 { n += 1; x >>= 7; }
    n
}

fn write_uleb(mut value: usize, out: &mut Vec<u8>) {
    while value >= 128 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn read_uleb(input: &[u8], cursor: &mut usize) -> Result<usize, Error> {
    let start = *cursor;
    let mut n = 0u64;
    for step in 0..5 {
        let byte = *input.get(*cursor).ok_or(Error::Truncated)?;
        *cursor += 1;
        n |= u64::from(byte & 127) << (7 * step);
        if byte & 128 == 0 {
            let value = usize::try_from(n).map_err(|_| Error::TooLarge)?;
            if *cursor - start != uleb_len(value) { return Err(Error::NonCanonical); }
            return Ok(value);
        }
    }
    Err(Error::Malformed)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut result = !0u32;
    for &byte in bytes {
        result ^= u32::from(byte);
        for _ in 0..8 {
            result = (result >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(result & 1)));
        }
    }
    !result
}

fn put_descriptor(word: &Word, out: &mut Vec<u8>) {
    match word {
        Word::Core(w) => out.push(w.width() as u8),
        Word::D10(_) => out.push(10),
        Word::Numeric { level, .. } => {
            out.push(NUMERIC_DESCRIPTOR);
            write_uleb(*level as usize, out);
        }
    }
}

fn descriptor(input: &[u8], at: &mut usize) -> Result<Word, Error> {
    let kind = *input.get(*at).ok_or(Error::Truncated)?;
    *at += 1;
    match kind {
        1..=9 => Ok(Word::Core(ExactWord::new(kind, 0)
            .map_err(|_| Error::Malformed)?.to_binary_source())),
        10 => Ok(Word::D10(DomainD10::new(0).expect("valid ten-bit D10 placeholder"))),
        NUMERIC_DESCRIPTOR => {
            let level = read_uleb(input, at)?;
            let level = u8::try_from(level).map_err(|_| Error::TooLarge)?;
            Ok(Word::Numeric { level, bytes: vec![0; numeric_bytes(level)?] })
        }
        _ => Err(Error::Malformed),
    }
}

fn word_bit(word: &Word, bit: usize) -> u8 {
    match word {
        Word::Core(w) => ((w.packed_bits() >> (w.width() - 1 - bit)) & 1) as u8,
        Word::D10(w) => ((w.packed_bits() >> (9 - bit)) & 1) as u8,
        Word::Numeric { bytes, .. } => (bytes[bit / 8] >> (7 - bit % 8)) & 1,
    }
}

fn write_packed(words: &[Word], out: &mut Vec<u8>) {
    let width: usize = words.iter().map(Word::width_bits).sum();
    let start = out.len();
    out.resize(start + width.div_ceil(8), 0);
    let mut offset = 0usize;
    for word in words {
        for bit in 0..word.width_bits() {
            if word_bit(word, bit) == 1 { out[start + offset / 8] |= 1 << (7 - offset % 8); }
            offset += 1;
        }
    }
}

fn read_packed(templates: &mut [Word], input: &[u8], pos: &mut usize) -> Result<(), Error> {
    let total: usize = templates.iter().map(Word::width_bits).sum();
    let end = pos.checked_add(total.div_ceil(8)).ok_or(Error::TooLarge)?;
    if end > input.len() { return Err(Error::Truncated); }
    let body = &input[*pos..end];
    if !total.is_multiple_of(8) && !body.is_empty() {
        let mask = (1 << (8 - total % 8)) - 1;
        if body[body.len() - 1] & mask != 0 { return Err(Error::NonCanonical); }
    }
    let mut bit_pos = 0;
    for template in templates.iter_mut() {
        match template {
            Word::Core(old) => {
                let width = old.width();
                let mut value = 0u16;
                for _ in 0..width {
                    value = (value << 1) | u16::from((body[bit_pos / 8] >> (7 - bit_pos % 8)) & 1);
                    bit_pos += 1;
                }
                *old = ExactWord::new(width as u8, value)
                    .map_err(|_| Error::Malformed)?.to_binary_source();
            }
            Word::D10(old) => {
                let mut value = 0u16;
                for _ in 0..10 {
                    value = (value << 1) | u16::from((body[bit_pos / 8] >> (7 - bit_pos % 8)) & 1);
                    bit_pos += 1;
                }
                *old = DomainD10::new(value).ok_or(Error::Malformed)?;
            }
            Word::Numeric { bytes, .. } => {
                for byte in bytes.iter_mut() {
                    let mut value = 0u8;
                    for _ in 0..8 {
                        value = (value << 1) | ((body[bit_pos / 8] >> (7 - bit_pos % 8)) & 1);
                        bit_pos += 1;
                    }
                    *byte = value;
                }
            }
        }
    }
    *pos = end;
    Ok(())
}

/// The canonical reader is the structural authority for D2. An explicit
/// Numeric is an atomic expression for *structural validation only*: the
/// substituted W1 is never exposed as a semantic identity or encoded byte.
/// No claim about an executable numeric operation follows from this check.
fn valid_structure(words: &[Word]) -> Result<(), Error> {
    let surrogate = words.iter().map(|w| match w {
        Word::Core(core) => *core,
        Word::D10(_) | Word::Numeric { .. } => BinarySourceWord::W1(Bit1::new(0).unwrap()),
    }).collect::<Vec<_>>();
    parse_canonical_word_sequence(&surrogate).map(|_| ()).map_err(|_| Error::InvalidD2)
}

fn grammar(words: &[Word]) -> Option<(usize, u128, usize)> {
    if words.len() < 2 { return None; }
    let core = words.iter().map(|w| match w { Word::Core(x) => Some(*x), _ => None })
        .collect::<Option<Vec<_>>>()?;
    let inner_bits = core.get(1..core.len() - 1)?.iter().map(|w| w.width()).sum();
    if inner_bits > MAX_INNER_BITS { return None; }
    let ordinal = rank_binary_source_words(&core).ok()?.ordinal;
    let count = count_inner(inner_bits).ok()?;
    let required_bits = (128 - (count - 1).leading_zeros()) as usize;
    Some((inner_bits, ordinal, required_bits.div_ceil(8)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Mode { Raw, Repeat, NumericRun, Grammar }

#[derive(Clone, Copy)]
struct Step { cost: usize, end: usize, mode: Mode }

fn plan(words: &[Word]) -> Vec<Step> {
    let n = words.len();
    let mut dp = vec![Step { cost: usize::MAX, end: n, mode: Mode::Raw }; n + 1];
    dp[n].cost = 0;
    for i in (0..n).rev() {
        let mut payload_bits = 0usize;
        let mut desc_bytes = 0usize;
        let mut all_repeat = true;
        let mut same_numeric = None;
        for j in i + 1..=n.min(i + MAX_BLOCK_WORDS) {
            let current = &words[j - 1];
            payload_bits += current.width_bits();
            desc_bytes += current.descriptor_cost();
            all_repeat &= current == &words[i];
            let level = match current { Word::Numeric { level, .. } => Some(*level), _ => None };
            if j == i + 1 { same_numeric = level; }
            else if same_numeric != level { same_numeric = None; }
            let count = j - i;
            let overhead = 1 + uleb_len(count);
            let mut options = vec![(overhead + desc_bytes + payload_bits.div_ceil(8), Mode::Raw)];
            if all_repeat {
                options.push((overhead + words[i].descriptor_cost() + words[i].width_bits().div_ceil(8), Mode::Repeat));
            }
            if let Some(level) = same_numeric {
                options.push((overhead + uleb_len(level as usize) + payload_bits.div_ceil(8), Mode::NumericRun));
            }
            // The only ranked profile whose exact D2 population is independently proved.
            if payload_bits <= MAX_INNER_BITS + 4 {
                if let Some((inner_bits, _, physical)) = grammar(&words[i..j]) {
                    options.push((overhead + uleb_len(inner_bits) + physical, Mode::Grammar));
                }
            }
            for (block_cost, mode) in options {
                let total = block_cost + dp[j].cost;
                let old = dp[i];
                if total < old.cost ||
                    (total == old.cost && (mode, usize::MAX - j) < (old.mode, usize::MAX - old.end)) {
                    dp[i] = Step { cost: total, end: j, mode };
                }
            }
        }
    }
    dp
}

pub fn encode(words: &[Word]) -> Result<Vec<u8>, Error> {
    if words.len() > MAX_WORDS { return Err(Error::TooLarge); }
    let mut total_bits = 0usize;
    for word in words {
        if let Word::Numeric { level, bytes } = word {
            if bytes.len() != numeric_bytes(*level)? { return Err(Error::Malformed); }
        }
        total_bits = total_bits.checked_add(word.width_bits()).ok_or(Error::TooLarge)?;
    }
    if total_bits.div_ceil(8) > MAX_BYTES { return Err(Error::TooLarge); }
    valid_structure(words)?;
    let steps = plan(words);
    let mut output = Vec::new();
    output.extend_from_slice(&MAGIC);
    write_uleb(words.len(), &mut output);
    let mut i = 0;
    while i < words.len() {
        let step = steps[i];
        let block = &words[i..step.end];
        output.push(step.mode as u8);
        write_uleb(block.len(), &mut output);
        match step.mode {
            Mode::Raw => {
                for word in block { put_descriptor(word, &mut output); }
                write_packed(block, &mut output);
            }
            Mode::Repeat => {
                put_descriptor(&block[0], &mut output);
                write_packed(&block[..1], &mut output);
            }
            Mode::NumericRun => {
                let Word::Numeric { level, .. } = &block[0] else { return Err(Error::Malformed); };
                write_uleb(*level as usize, &mut output);
                write_packed(block, &mut output);
            }
            Mode::Grammar => {
                let (inner, mut ordinal, size) = grammar(block).ok_or(Error::Malformed)?;
                write_uleb(inner, &mut output);
                let begin = output.len();
                output.resize(begin + size, 0);
                for byte in output[begin..].iter_mut().rev() { *byte = ordinal as u8; ordinal >>= 8; }
                if ordinal != 0 { return Err(Error::Malformed); }
            }
        }
        if output.len() > MAX_BYTES - 4 { return Err(Error::TooLarge); }
        i = step.end;
    }
    let checksum = crc32(&output);
    output.extend_from_slice(&checksum.to_be_bytes());
    if output.len() > MAX_BYTES { return Err(Error::TooLarge); }
    Ok(output)
}

pub fn decode(input: &[u8]) -> Result<Vec<Word>, Error> {
    if input.len() > MAX_BYTES { return Err(Error::TooLarge); }
    if input.len() < 8 { return Err(Error::Truncated); }
    if input[..3] != MAGIC { return Err(Error::BadVersion); }
    let end = input.len() - 4;
    let expected = u32::from_be_bytes(input[end..].try_into().map_err(|_| Error::Truncated)?);
    if expected != crc32(&input[..end]) { return Err(Error::Integrity); }
    let mut pos = 3;
    let words_count = read_uleb(&input[..end], &mut pos)?;
    if words_count > MAX_WORDS { return Err(Error::TooLarge); }
    let mut result = Vec::with_capacity(words_count);
    let mut decoded_bits = 0usize;
    while result.len() < words_count {
        let mode = *input[..end].get(pos).ok_or(Error::Truncated)?;
        pos += 1;
        let count = read_uleb(&input[..end], &mut pos)?;
        if count == 0 || count > MAX_BLOCK_WORDS || count > words_count - result.len() {
            return Err(Error::Malformed);
        }
        let mut block = match mode {
            RAW => {
                let mut items = Vec::with_capacity(count);
                let mut pending_bits = 0usize;
                for _ in 0..count {
                    let item = descriptor(&input[..end], &mut pos)?;
                    pending_bits = pending_bits.checked_add(item.width_bits()).ok_or(Error::TooLarge)?;
                    if pending_bits > MAX_BYTES * 8 - decoded_bits { return Err(Error::TooLarge); }
                    items.push(item);
                }
                items
            }
            REPEAT => {
                let item = descriptor(&input[..end], &mut pos)?;
                if item.width_bits().checked_mul(count).ok_or(Error::TooLarge)?
                    > MAX_BYTES * 8 - decoded_bits { return Err(Error::TooLarge); }
                vec![item]
            },
            NUMERIC_RUN => {
                let level = read_uleb(&input[..end], &mut pos)?;
                let level = u8::try_from(level).map_err(|_| Error::TooLarge)?;
                let size = numeric_bytes(level)?;
                if size.checked_mul(count).ok_or(Error::TooLarge)? > MAX_BYTES {
                    return Err(Error::TooLarge);
                }
                vec![Word::Numeric { level, bytes: vec![0; size] }; count]
            }
            GRAMMAR => {
                let inner = read_uleb(&input[..end], &mut pos)?;
                let cap = count_inner(inner).map_err(|_| Error::Malformed)?;
                let size = ((128 - (cap - 1).leading_zeros()) as usize).div_ceil(8);
                let stop = pos.checked_add(size).ok_or(Error::TooLarge)?;
                if stop > end { return Err(Error::Truncated); }
                let mut ordinal = 0u128;
                for &byte in &input[pos..stop] { ordinal = (ordinal << 8) | u128::from(byte); }
                pos = stop;
                let core = unrank_binary_source_words(inner, ordinal)
                    .map_err(|_| Error::NonCanonical)?;
                if core.len() != count { return Err(Error::NonCanonical); }
                let recovered_bits: usize = core.iter().map(|word| word.width()).sum();
                decoded_bits = decoded_bits.checked_add(recovered_bits).ok_or(Error::TooLarge)?;
                if decoded_bits.div_ceil(8) > MAX_BYTES { return Err(Error::TooLarge); }
                result.extend(core.into_iter().map(Word::Core));
                continue;
            }
            _ => return Err(Error::Malformed),
        };
        let block_bits = block.iter().map(Word::width_bits).sum::<usize>();
        let expanded_bits = block_bits.checked_mul(if mode == REPEAT { count } else { 1 })
            .ok_or(Error::TooLarge)?;
        decoded_bits = decoded_bits.checked_add(expanded_bits).ok_or(Error::TooLarge)?;
        if decoded_bits.div_ceil(8) > MAX_BYTES { return Err(Error::TooLarge); }
        read_packed(&mut block, &input[..end], &mut pos)?;
        if mode == REPEAT { result.resize(result.len() + count, block[0].clone()); }
        else { result.extend(block); }
    }
    if pos != end { return Err(Error::NonCanonical); }
    valid_structure(&result)?;
    // Reject all alternative block partitions, width-map aliases and padding.
    if encode(&result)?.as_slice() != input { return Err(Error::NonCanonical); }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_binary_source_words;

    fn core(source: &str) -> Vec<Word> {
        parse_binary_source_words(source).unwrap().into_iter().map(|x| Word::Core(x.word)).collect()
    }
    fn check(words: Vec<Word>) -> Vec<u8> {
        let physical = encode(&words).unwrap();
        assert_eq!(decode(&physical).unwrap(), words);
        assert_eq!(encode(&decode(&physical).unwrap()).unwrap(), physical);
        for i in 0..physical.len() { assert!(decode(&physical[..i]).is_err()); }
        let mut extra = physical.clone(); extra.push(0);
        assert!(decode(&extra).is_err());
        physical
    }

    #[test]
    fn small_core_and_grammar_are_byte_exact() {
        for source in ["", "0", "1", "10 01", "10 001 00 000 01", "10 01 10 01", "10 0 11 1 01"] {
            let bytes = check(core(source));
            assert_eq!(&bytes[..3], &MAGIC);
        }
        let sample = check(core("10 001 00 000 01"));
        assert_eq!(sample[4], GRAMMAR);
    }

    #[test]
    fn equal_magnitude_d24_and_d48_are_distinct_and_unchanged() {
        let one24 = Word::numeric(0, { let mut b = vec![0; 3]; b[2] = 1; b }).unwrap();
        let one48 = Word::numeric(1, { let mut b = vec![0; 6]; b[5] = 1; b }).unwrap();
        assert_ne!(one24, one48);
        let mut source = core("10 001 00");
        source.push(one24.clone());
        source.extend(core("00"));
        source.push(one48.clone());
        source.extend(core("01"));
        check(source);
        assert!(Word::numeric(0, vec![1; 6]).is_err());
        assert!(Word::numeric(1, vec![1; 3]).is_err());
    }

    #[test]
    fn numeric_runs_and_repeats_save_physical_bytes() {
        let one24 = Word::numeric(0, vec![0xff; 3]).unwrap();
        let repeated = vec![one24.clone(); 32];
        let bytes = check(repeated);
        assert_eq!(bytes[4], REPEAT);
        let numbers = (0..20u8).map(|v| Word::numeric(0, vec![0, 0, v]).unwrap()).collect::<Vec<_>>();
        let bytes = check(numbers);
        assert_eq!(bytes[4], NUMERIC_RUN);
    }

    #[test]
    fn independently_pinned_non_grammar_wire_vectors() {
        // Produced by the standalone Python v3 reference; no Rust encoder
        // is used to create these expected physical bytes.
        let samples: &[(&[Word], &[u8])] = &[
            (&[], &[0x53, 0x33, 0x03, 0x00, 0xf5, 0xfc, 0x77, 0x5a]),
        ];
        for (words, expected) in samples {
            assert_eq!(encode(words).unwrap().as_slice(), *expected);
            assert_eq!(decode(expected).unwrap().as_slice(), *words);
        }
        let one24 = Word::numeric(0, vec![0, 0, 1]).unwrap();
        let one48 = Word::numeric(1, vec![0, 0, 0, 0, 0, 1]).unwrap();
        let pairs: &[(Vec<Word>, &[u8])] = &[
            (vec![one24.clone()], &[0x53,0x33,0x03,0x01,0x02,0x01,0x00,0x00,0x00,0x01,0x89,0x42,0x1d,0x50]),
            (vec![one48], &[0x53,0x33,0x03,0x01,0x02,0x01,0x01,0x00,0x00,0x00,0x00,0x00,0x01,0x9d,0x8f,0xe7,0xcc]),
            (vec![one24; 5], &[0x53,0x33,0x03,0x05,0x01,0x05,0x80,0x00,0x00,0x00,0x01,0x48,0x29,0x1b,0x50]),
        ];
        for (words, expected) in pairs {
            assert_eq!(encode(words).unwrap().as_slice(), *expected);
            assert_eq!(decode(expected).unwrap().as_slice(), words.as_slice());
        }
    }

    #[test]
    fn numeric_levels_above_small_storage_tiers_remain_exact() {
        // The resource limit is not a language-domain ceiling.  D24 * 2^13
        // is larger than the old candidate's level-12 cap.
        for level in [7, 12, 13, 15] {
            let mut data = vec![0u8; numeric_bytes(level).unwrap()];
            data[0] = 0x80;
            *data.last_mut().unwrap() = 0x01;
            let value = Word::numeric(level, data).unwrap();
            let physical = encode(&[value.clone()]).unwrap();
            assert_eq!(decode(&physical).unwrap(), vec![value]);
        }
        assert_eq!(numeric_bytes(MAX_NUMERIC_LEVEL + 1), Err(Error::TooLarge));
    }

    #[test]
    fn oversized_numeric_raw_is_rejected_before_allocating_all_descriptors() {
        // Craft a correctly check-summed RAW with two width-18 descriptors.
        // The declared expansion cannot fit the MAX_BYTES decoded budget.
        let mut bytes = Vec::from(MAGIC);
        write_uleb(2, &mut bytes);
        bytes.push(RAW);
        write_uleb(2, &mut bytes);
        for _ in 0..2 {
            bytes.push(NUMERIC_DESCRIPTOR);
            write_uleb(MAX_NUMERIC_LEVEL as usize, &mut bytes);
        }
        let checksum = crc32(&bytes);
        bytes.extend_from_slice(&checksum.to_be_bytes());
        assert_eq!(decode(&bytes), Err(Error::TooLarge));
    }

    #[test]
    fn wide_values_and_wrong_domain_or_corrupted_frames_are_rejected() {
        for level in 0..=4 {
            let mut bits = vec![0; numeric_bytes(level).unwrap()];
            bits[0] = 0x80; let last = bits.len() - 1; bits[last] = 0x55;
            check(vec![Word::numeric(level, bits).unwrap()]);
        }
        assert_eq!(Word::numeric(MAX_NUMERIC_LEVEL + 1, vec![]), Err(Error::TooLarge));
        let good = check(core("10 001 01"));
        let mut mutated = good.clone(); mutated[3] = 0x80;
        assert!(decode(&mutated).is_err());
        let mut changed = good.clone(); changed[2] = 1;
        assert_eq!(decode(&changed), Err(Error::BadVersion));
    }


    /// Вичерпний свідок 1024/1024 координат D10: разом із D2/D3/D8 вони
    /// займають точно 25 змістовних бітів, не 32, без внутрішніх padding.
    #[test]
    fn d10_all_coordinates_have_exact_ten_bit_payload_and_distinct_d8_identity() {
        let open = Word::Core(BinarySourceWord::W2(crate::Bit2::new(0b10).unwrap()));
        let close = Word::Core(BinarySourceWord::W2(crate::Bit2::new(0b01).unwrap()));
        let quote = Word::Core(BinarySourceWord::W3(crate::Bit3::new(0b001).unwrap()));
        let d8 = Word::Core(BinarySourceWord::W8(crate::Bit8::new(0b1010_1000).unwrap()));
        for payload in 0u16..1024 {
            let d10 = Word::D10(DomainD10::new(payload).unwrap());
            assert_eq!(d10.width_bits(), 10);
            assert_ne!(d10, Word::Core(BinarySourceWord::W8(
                crate::Bit8::new((payload & 0xff) as u8).unwrap()
            )));
            let sequence = vec![open.clone(), quote.clone(), d10.clone(), d8.clone(), close.clone()];
            let mut packed = Vec::new();
            write_packed(&sequence, &mut packed);
            assert_eq!(packed.len(), 4);
            assert_eq!(packed[3] & 0x7f, 0, "лише кінцевий padding дозволено");
            // Незалежний текстовий оракул, а не повторення реалізації write_packed.
            let expected = format!("10001{payload:010b}1010100001");
            assert_eq!(expected.len(), 25);
            for (offset, expected_bit) in expected.bytes().enumerate() {
                assert_eq!((packed[offset / 8] >> (7 - offset % 8)) & 1, expected_bit - b'0',
                    "D10={payload}, зсув={offset}");
            }
            let encoded = encode(&sequence).expect("канонічний E3 має приймати D10");
            assert_eq!(decode(&encoded).unwrap(), sequence);
        }
        assert_eq!(DomainD10::new(1024), None);
        assert_eq!(DomainD10::new(u16::MAX), None);
    }

    #[test]
    fn d10_descriptor_and_padding_are_canonical_without_implicit_callability() {
        let d10 = Word::D10(DomainD10::new(0b1010_1010_10).unwrap());
        assert_eq!(d10.descriptor_cost(), 1);
        let one = vec![d10.clone()];
        let encoded = encode(&one).unwrap();
        assert_eq!(decode(&encoded).unwrap(), one);
        // Явний RAW-дескриптор 10, незалежний від вибору DP-планувальника.
        let mut raw = MAGIC.to_vec();
        write_uleb(1, &mut raw);
        raw.push(RAW);
        write_uleb(1, &mut raw);
        raw.push(10);
        raw.extend_from_slice(&[0xaa, 0x80]);
        let checksum = crc32(&raw);
        raw.extend_from_slice(&checksum.to_be_bytes());
        assert_eq!(decode(&raw).unwrap(), one);
        // Канонічний CRC не дозволяє приховати ненульовий padding.
        let mut bad_padding = raw[..raw.len() - 4].to_vec();
        *bad_padding.last_mut().unwrap() |= 1;
        let checksum = crc32(&bad_padding);
        bad_padding.extend_from_slice(&checksum.to_be_bytes());
        assert_eq!(decode(&bad_padding), Err(Error::NonCanonical));
        for invalid in [11u8, 0x7f] {
            let mut wrong = raw[..raw.len() - 4].to_vec();
            wrong[6] = invalid;
            let checksum = crc32(&wrong);
            wrong.extend_from_slice(&checksum.to_be_bytes());
            assert_eq!(decode(&wrong), Err(Error::Malformed));
        }
    }

    #[test]
    fn no_numeric_width_can_steal_d2_structure() {
        let good = vec![Word::Core(crate::BinarySourceWord::W2(crate::Bit2::new(0b10).unwrap())),
            Word::numeric(0, vec![0; 3]).unwrap(),
            Word::Core(crate::BinarySourceWord::W2(crate::Bit2::new(0b01).unwrap()))];
        check(good);
        let bad = vec![Word::Core(crate::BinarySourceWord::W2(crate::Bit2::new(0b01).unwrap())),
            Word::numeric(0, vec![0; 3]).unwrap()];
        assert_eq!(encode(&bad), Err(Error::InvalidD2));
    }
}
