//! Exact-word D2 grammar rank/unrank for Universal Frame3 research.
//!
//! This is a candidate for a future explicitly versioned physical profile,
//! NOT a new .sens default, callability oracle, or entire-file byte optimum.
//! The exterior words are fixed W2:10 ... W2:01. Every interior W2 (including
//! every 00 separator) is preserved, with exact widths/bits for W1 and W3-W9.
//! The decoder MUST obtain the exact interior bit count from trusted framing.
//! This implementation is intentionally bounded to <=20 interior bits.
//!
//! Source proof: research/framed3/canonical_d2_rank_research.py and the
//! independent canonical reader tests. It does not assume that every parsed
//! list is executable: ((ATOM)) as code needs semantic rejection, whereas
//! (QUOTE ((ATOM))) needs to remain representable as data.

use std::cell::RefCell;
use std::collections::HashMap;

pub const MAX_INNER_BITS: usize = 20;

const EMPTY: u8 = 0;
const ITEMS: u8 = 1;
const DOT_TAIL: u8 = 2;
const DOT_CLOSE: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ExactWord {
    width: u8,
    bits: u16,
}

impl ExactWord {
    pub fn new(width: u8, bits: u16) -> Result<Self, GrammarRankError> {
        if !(1..=9).contains(&width) || bits >= (1u16 << width) {
            return Err(GrammarRankError::InvalidWord);
        }
        Ok(Self { width, bits })
    }

    pub fn width(self) -> u8 {
        self.width
    }

    pub fn bits(self) -> u16 {
        self.bits
    }

    /// Direct typed codec handoff; never creates a textual 0/1 projection.
    pub fn from_binary_source(word: crate::BinarySourceWord) -> Self {
        Self {
            width: word.width() as u8,
            bits: word.packed_bits(),
        }
    }

    /// Exact typed restoration, with no lossy 8-bit cast for W9.
    pub fn to_binary_source(self) -> crate::BinarySourceWord {
        use crate::{
            BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9,
        };
        // Private fields and the checked constructor make these valid.
        match self.width {
            1 => BinarySourceWord::W1(Bit1::new(self.bits as u8).unwrap()),
            2 => BinarySourceWord::W2(Bit2::new(self.bits as u8).unwrap()),
            3 => BinarySourceWord::W3(Bit3::new(self.bits as u8).unwrap()),
            4 => BinarySourceWord::W4(Bit4::new(self.bits as u8).unwrap()),
            5 => BinarySourceWord::W5(Bit5::new(self.bits as u8).unwrap()),
            6 => BinarySourceWord::W6(Bit6::new(self.bits as u8).unwrap()),
            7 => BinarySourceWord::W7(Bit7::new(self.bits as u8).unwrap()),
            8 => BinarySourceWord::W8(Bit8::new(self.bits as u8).unwrap()),
            9 => BinarySourceWord::W9(Bit9::new(self.bits).unwrap()),
            _ => unreachable!("checked exact word width"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrammarRankError {
    TooLarge,
    InvalidWord,
    InvalidD2,
    OutOfRange,
    NonCanonical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrammarRank {
    pub inner_bits: u8,
    pub ordinal: u128,
}

fn root() -> Vec<u8> {
    vec![EMPTY]
}

fn complete_expression(stack: &[u8]) -> Option<Vec<u8>> {
    if stack.len() <= 1 {
        return None;
    }
    let parent = stack[stack.len() - 2];
    let replacement = match parent {
        EMPTY | ITEMS => ITEMS,
        DOT_TAIL => DOT_CLOSE,
        _ => return None,
    };
    let mut result = stack[..stack.len() - 2].to_vec();
    result.push(replacement);
    Some(result)
}

/// A D2-aware state transition. A W2 word is NEVER a payload atom.
fn advance(stack: &[u8], word: ExactWord) -> Option<Vec<u8>> {
    let state = *stack.last()?;
    if word.width == 2 {
        return match word.bits {
            0 => Some(stack.to_vec()), // 00 is an exact, potentially repeated separator.
            1 if state != DOT_TAIL => complete_expression(stack),
            1 => None,
            2 if state != DOT_CLOSE => {
                let mut next = stack.to_vec();
                next.push(EMPTY);
                Some(next)
            }
            2 => None,
            3 if state == ITEMS => {
                let mut next = stack.to_vec();
                *next.last_mut()? = DOT_TAIL;
                Some(next)
            }
            3 => None,
            _ => None,
        };
    }

    let next_phase = match state {
        EMPTY | ITEMS => ITEMS,
        DOT_TAIL => DOT_CLOSE,
        _ => return None,
    };
    let mut next = stack.to_vec();
    *next.last_mut()? = next_phase;
    Some(next)
}

fn accepts(stack: &[u8]) -> bool {
    stack.len() == 1 && matches!(stack[0], EMPTY | ITEMS | DOT_CLOSE)
}

type Memo = HashMap<(usize, Vec<u8>), u128>;

// Only proven small D2 state/count tables (at most 20 inner bits) are
// retained. Thread-local memoization avoids mutex contention in codecs and
// repeats neither DP enumeration nor host-side runtime capabilities.
thread_local! {
    static GRAMMAR_COUNTS: RefCell<Memo> = RefCell::new(HashMap::new());
}

fn cached_count_suffix(remaining: usize, stack: &[u8]) -> u128 {
    GRAMMAR_COUNTS.with(|memo| {
        count_suffix(remaining, stack, &mut memo.borrow_mut())
    })
}


fn count_suffix(remaining: usize, stack: &[u8], memo: &mut Memo) -> u128 {
    if remaining == 0 {
        return u128::from(accepts(stack));
    }
    let key = (remaining, stack.to_vec());
    if let Some(&cached) = memo.get(&key) {
        return cached;
    }
    let mut total = 0u128;
    for width in 1..=remaining.min(9) {
        if width == 2 {
            for value in 0..4 {
                let word = ExactWord {
                    width: 2,
                    bits: value,
                };
                if let Some(next) = advance(stack, word) {
                    total += count_suffix(remaining - 2, &next, memo);
                }
            }
        } else {
            // All 2^width payload values have identical D2 transitions.
            let representative = ExactWord {
                width: width as u8,
                bits: 0,
            };
            if let Some(next) = advance(stack, representative) {
                total += (1u128 << width) * count_suffix(remaining - width, &next, memo);
            }
        }
    }
    memo.insert(key, total);
    total
}

/// A ready-to-use bridge for the existing Universal Frame3 typed word API.
pub fn rank_binary_source_words(
    words: &[crate::BinarySourceWord],
) -> Result<GrammarRank, GrammarRankError> {
    let exact = words
        .iter()
        .copied()
        .map(ExactWord::from_binary_source)
        .collect::<Vec<_>>();
    rank_single_root(&exact)
}

/// Reconstruct the original typed words, without symbol/name reinterpretation.
pub fn unrank_binary_source_words(
    inner_bits: usize,
    ordinal: u128,
) -> Result<Vec<crate::BinarySourceWord>, GrammarRankError> {
    unrank_single_root(inner_bits, ordinal).map(|words| {
        words.into_iter().map(ExactWord::to_binary_source).collect()
    })
}

pub fn count_inner(inner_bits: usize) -> Result<u128, GrammarRankError> {
    if inner_bits > MAX_INNER_BITS {
        return Err(GrammarRankError::TooLarge);
    }
    Ok(cached_count_suffix(inner_bits, &root()))
}

/// Reversible 0-based ordinal of one complete D2 root and its exact words.
/// This does NOT test semantic validity as an executable SENS program.
pub fn rank_single_root(words: &[ExactWord]) -> Result<GrammarRank, GrammarRankError> {
    let boundary = ExactWord { width: 2, bits: 2 };
    let end = ExactWord { width: 2, bits: 1 };
    if words.len() < 2 || words.first() != Some(&boundary) || words.last() != Some(&end) {
        return Err(GrammarRankError::InvalidD2);
    }
    if words.iter().any(|word| ExactWord::new(word.width, word.bits).is_err()) {
        return Err(GrammarRankError::InvalidWord);
    }
    let inner = &words[1..words.len() - 1];
    let bits = inner.iter().map(|word| usize::from(word.width)).sum::<usize>();
    let capacity = count_inner(bits)?;
    let mut remaining = bits;
    let mut state = root();
    let mut ordinal = 0u128;

    for word in inner {
        let mut found = false;
        'widths: for width in 1..=remaining.min(9) {
            if width == 2 {
                // All D2 words are distinct structural transitions.
                for value in 0..4 {
                    let candidate = ExactWord { width: 2, bits: value };
                    let Some(next) = advance(&state, candidate) else {
                        continue;
                    };
                    let suffixes = cached_count_suffix(remaining - width, &next);
                    if suffixes == 0 {
                        continue;
                    }
                    if candidate == *word {
                        state = next;
                        remaining -= width;
                        found = true;
                        break 'widths;
                    }
                    ordinal += suffixes;
                }
            } else {
                // All 2^width non-D2 values have the SAME successor and
                // completion count. Count the earlier numeric coordinates
                // algebraically instead of enumerating up to 512 choices.
                let representative = ExactWord { width: width as u8, bits: 0 };
                let Some(next) = advance(&state, representative) else {
                    continue;
                };
                let suffixes = cached_count_suffix(remaining - width, &next);
                if suffixes == 0 {
                    continue;
                }
                if word.width == width as u8 {
                    ordinal += u128::from(word.bits) * suffixes;
                    state = next;
                    remaining -= width;
                    found = true;
                    break 'widths;
                }
                ordinal += (1u128 << width) * suffixes;
            }
        }
        if !found {
            return Err(GrammarRankError::InvalidD2);
        }
    }
    if remaining != 0 || !accepts(&state) || ordinal >= capacity {
        return Err(GrammarRankError::InvalidD2);
    }
    Ok(GrammarRank {
        inner_bits: bits as u8,
        ordinal,
    })
}

pub fn unrank_single_root(
    inner_bits: usize,
    mut ordinal: u128,
) -> Result<Vec<ExactWord>, GrammarRankError> {
    let capacity = count_inner(inner_bits)?;
    if ordinal >= capacity {
        return Err(GrammarRankError::OutOfRange);
    }
    let mut remaining = inner_bits;
    let mut state = root();
    let mut words = vec![ExactWord { width: 2, bits: 2 }];

    while remaining > 0 {
        let mut found = false;
        'widths: for width in 1..=remaining.min(9) {
            if width == 2 {
                for value in 0..4 {
                    let candidate = ExactWord { width: 2, bits: value };
                    let Some(next) = advance(&state, candidate) else {
                        continue;
                    };
                    let suffixes = cached_count_suffix(remaining - width, &next);
                    if suffixes == 0 {
                        continue;
                    }
                    if ordinal < suffixes {
                        words.push(candidate);
                        state = next;
                        remaining -= width;
                        found = true;
                        break 'widths;
                    }
                    ordinal -= suffixes;
                }
            } else {
                let representative = ExactWord { width: width as u8, bits: 0 };
                let Some(next) = advance(&state, representative) else {
                    continue;
                };
                let suffixes = cached_count_suffix(remaining - width, &next);
                if suffixes == 0 {
                    continue;
                }
                let group = (1u128 << width) * suffixes;
                if ordinal < group {
                    let value = (ordinal / suffixes) as u16;
                    ordinal %= suffixes;
                    words.push(ExactWord { width: width as u8, bits: value });
                    state = next;
                    remaining -= width;
                    found = true;
                    break 'widths;
                }
                ordinal -= group;
            }
        }
        if !found {
            return Err(GrammarRankError::InvalidD2);
        }
    }

    if !accepts(&state) || ordinal != 0 {
        return Err(GrammarRankError::InvalidD2);
    }
    words.push(ExactWord { width: 2, bits: 1 });
    Ok(words)
}

fn exact_body_bytes(bits: usize) -> Result<usize, GrammarRankError> {
    let cap = count_inner(bits)?;
    // Minimal fixed-width integer representation of [0, cap) at known bits.
    let required_bits = (u128::BITS - (cap - 1).leading_zeros()) as usize;
    Ok(required_bits.div_ceil(8))
}

/// The returned body bytes have NO file length, profile, version or CRC.
/// They are NOT a valid .sens file and are never passed to the T5 decoder.
pub fn encode_body_at_known_length(
    words: &[ExactWord],
) -> Result<(usize, Vec<u8>), GrammarRankError> {
    let ranked = rank_single_root(words)?;
    let bits = usize::from(ranked.inner_bits);
    let mut number = ranked.ordinal;
    let mut body = vec![0; exact_body_bytes(bits)?];
    for byte in body.iter_mut().rev() {
        *byte = number as u8;
        number >>= 8;
    }
    Ok((bits, body))
}

/// Rejects unused, short/long and noncanonical integer codes.
pub fn decode_body_at_known_length(
    inner_bits: usize,
    body: &[u8],
) -> Result<Vec<ExactWord>, GrammarRankError> {
    if body.len() != exact_body_bytes(inner_bits)? {
        return Err(GrammarRankError::NonCanonical);
    }
    let mut number = 0u128;
    for &byte in body {
        number = (number << 8) | u128::from(byte);
    }
    unrank_single_root(inner_bits, number)
}
