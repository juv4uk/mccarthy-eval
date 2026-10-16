//! #2265 compact boundary-index tournament for mixed exact-width payloads.
//!
//! Research-only mechanism benchmark. It does not choose wire framing, EOS,
//! semantic identities, or a production runtime representation.

use sens::{Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, BitPacker, PackedBitstream};
use std::{env, hint::black_box, mem::size_of, process::ExitCode};

#[derive(Clone, Copy)]
struct Word {
    width: u8,
    raw: u8,
}

fn mask(width: u8) -> u8 {
    if width == 8 {
        u8::MAX
    } else {
        ((1u16 << width) - 1) as u8
    }
}

fn fixed_width(case: &str) -> Option<u8> {
    match case {
        "w1" => Some(1),
        "w2" => Some(2),
        "w3" => Some(3),
        "w4" => Some(4),
        "w5" => Some(5),
        "w6" => Some(6),
        "w7" => Some(7),
        "w8" => Some(8),
        _ => None,
    }
}

fn width_at(case: &str, index: usize) -> Option<u8> {
    Some(match case {
        "w1" => 1,
        "w2" => 2,
        "w3" => 3,
        "w4" => 4,
        "w5" => 5,
        "w6" => 6,
        "w7" => 7,
        "w8" => 8,
        "d1234" => [2, 3, 2, 4, 2, 3, 1, 4, 2, 3, 4, 3][index % 12],
        "mixed18" => (index % 8 + 1) as u8,
        _ => return None,
    })
}

fn build_words(case: &str, count: usize) -> Option<Vec<Word>> {
    let mut words = Vec::with_capacity(count);
    let mut state = 0x9e37_79b9_u32 ^ count as u32;
    for index in 0..count {
        let width = width_at(case, index)?;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let raw = ((state as u8).wrapping_add(index as u8)) & mask(width);
        words.push(Word { width, raw });
    }
    Some(words)
}

fn push_word(packer: &mut BitPacker, word: Word) {
    match word.width {
        1 => { packer.push(Bit1::new(word.raw).unwrap()); }
        2 => { packer.push(Bit2::new(word.raw).unwrap()); }
        3 => { packer.push(Bit3::new(word.raw).unwrap()); }
        4 => { packer.push(Bit4::new(word.raw).unwrap()); }
        5 => { packer.push(Bit5::new(word.raw).unwrap()); }
        6 => { packer.push(Bit6::new(word.raw).unwrap()); }
        7 => { packer.push(Bit7::new(word.raw).unwrap()); }
        8 => { packer.push(Bit8::new(word.raw).unwrap()); }
        _ => unreachable!(),
    }
}

fn pack_words(words: &[Word]) -> PackedBitstream {
    let bits: usize = words.iter().map(|word| word.width as usize).sum();
    let mut packer = BitPacker::with_capacity_bits(bits);
    for &word in words {
        push_word(&mut packer, word);
    }
    packer.finish()
}

fn read_word(packed: &PackedBitstream, offset: usize, width: u8) -> u8 {
    match width {
        1 => packed.read::<1>(offset).unwrap().packed_bits(),
        2 => packed.read::<2>(offset).unwrap().packed_bits(),
        3 => packed.read::<3>(offset).unwrap().packed_bits(),
        4 => packed.read::<4>(offset).unwrap().packed_bits(),
        5 => packed.read::<5>(offset).unwrap().packed_bits(),
        6 => packed.read::<6>(offset).unwrap().packed_bits(),
        7 => packed.read::<7>(offset).unwrap().packed_bits(),
        8 => packed.read::<8>(offset).unwrap().packed_bits(),
        _ => unreachable!(),
    }
}

fn build_queries(count: usize, queries: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(queries);
    let mut state = 0x243f_6a88_u32 ^ count as u32 ^ queries as u32;
    for _ in 0..queries {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push((state as usize) % count);
    }
    out
}

fn total_bits(words: &[Word]) -> usize {
    words.iter().map(|word| word.width as usize).sum()
}

fn full_usize_offsets(words: &[Word]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(words.len() + 1);
    let mut bit = 0usize;
    offsets.push(bit);
    for word in words {
        bit += word.width as usize;
        offsets.push(bit);
    }
    offsets
}

fn full_u32_offsets(words: &[Word]) -> Vec<u32> {
    let total = total_bits(words);
    assert!(u32::try_from(total).is_ok(), "u32 offset control overflow");
    full_usize_offsets(words)
        .into_iter()
        .map(|offset| offset as u32)
        .collect()
}

fn checkpoint_offsets(words: &[Word], k: usize) -> Vec<u32> {
    assert!(k > 0);
    let mut checkpoints = Vec::with_capacity(words.len().div_ceil(k));
    let mut bit = 0usize;
    for (index, word) in words.iter().enumerate() {
        if index % k == 0 {
            checkpoints.push(u32::try_from(bit).expect("checkpoint offset fits u32"));
        }
        bit += word.width as usize;
    }
    checkpoints
}

fn block_local_offsets_u8(words: &[Word], k: usize) -> Option<Vec<u8>> {
    assert!(k > 0);
    let mut locals = Vec::with_capacity(words.len());
    let mut local_bit = 0usize;
    for (index, word) in words.iter().enumerate() {
        if index % k == 0 {
            local_bit = 0;
        }
        locals.push(u8::try_from(local_bit).ok()?);
        local_bit += word.width as usize;
    }
    Some(locals)
}

fn packed_width_stream2(words: &[Word]) -> Option<PackedBitstream> {
    let mut packer = BitPacker::with_capacity_bits(words.len() * 2);
    for word in words {
        if !(1..=4).contains(&word.width) {
            return None;
        }
        packer.push(Bit2::new(word.width - 1).unwrap());
    }
    Some(packer.finish())
}

fn packed_width2(widths: &PackedBitstream, index: usize) -> u8 {
    widths.read::<2>(index * 2).unwrap().packed_bits() + 1
}

fn packed_width_stream(words: &[Word]) -> PackedBitstream {
    let mut packer = BitPacker::with_capacity_bits(words.len() * 3);
    for word in words {
        packer.push(Bit3::new(word.width - 1).unwrap());
    }
    packer.finish()
}

fn packed_width(widths: &PackedBitstream, index: usize) -> u8 {
    widths.read::<3>(index * 3).unwrap().packed_bits() + 1
}

fn boundary_bitvector(words: &[Word], k: usize) -> (Vec<u64>, Vec<u32>) {
    assert!(k > 0);
    let total = total_bits(words);
    let mut bits = vec![0u64; total.div_ceil(64)];
    let mut checkpoints = Vec::with_capacity(words.len().div_ceil(k));
    let mut bit = 0usize;
    for (index, word) in words.iter().enumerate() {
        if index % k == 0 {
            checkpoints.push(u32::try_from(bit).expect("boundary checkpoint fits u32"));
        }
        bits[bit / 64] |= 1u64 << (bit % 64);
        bit += word.width as usize;
    }
    (bits, checkpoints)
}

fn nth_set_bit(mut word: u64, ordinal: usize) -> usize {
    debug_assert!(ordinal > 0);
    for _ in 1..ordinal {
        word &= word - 1;
    }
    word.trailing_zeros() as usize
}

fn select_boundary(
    bits: &[u64],
    checkpoints: &[u32],
    k: usize,
    index: usize,
) -> (usize, u64) {
    let block = index / k;
    let within = index % k;
    let checkpoint = checkpoints[block] as usize;
    if within == 0 {
        return (checkpoint, 0);
    }

    let mut remaining = within;
    let mut word_index = checkpoint / 64;
    let mut bit_in_word = checkpoint % 64 + 1;
    let mut probes = 0u64;

    loop {
        let mut word = bits[word_index];
        if bit_in_word < 64 {
            word &= u64::MAX << bit_in_word;
        } else {
            word = 0;
        }
        probes += 1;
        let count = word.count_ones() as usize;
        if remaining <= count {
            return (
                word_index * 64 + nth_set_bit(word, remaining),
                probes,
            );
        }
        remaining -= count;
        word_index += 1;
        bit_in_word = 0;
    }
}

fn next_boundary(bits: &[u64], start: usize, total: usize) -> (usize, u64) {
    let mut cursor = start + 1;
    let mut probes = 0u64;
    while cursor < total {
        let word_index = cursor / 64;
        let bit_in_word = cursor % 64;
        let word = bits[word_index] & (u64::MAX << bit_in_word);
        probes += 1;
        if word != 0 {
            return (
                word_index * 64 + word.trailing_zeros() as usize,
                probes,
            );
        }
        cursor = (word_index + 1) * 64;
    }
    (total, probes)
}

enum Index {
    Formula { width: u8 },
    FullUsize { offsets: Vec<usize> },
    FullU32 { offsets: Vec<u32> },
    CheckpointU8 {
        k: usize,
        checkpoints: Vec<u32>,
        widths: Vec<u8>,
    },
    CheckpointW2 {
        k: usize,
        checkpoints: Vec<u32>,
        widths: PackedBitstream,
    },
    CheckpointW3 {
        k: usize,
        checkpoints: Vec<u32>,
        widths: PackedBitstream,
    },
    TieredW2 {
        k: usize,
        checkpoints: Vec<u32>,
        locals: Vec<u8>,
        widths: PackedBitstream,
    },
    BoundarySelect {
        k: usize,
        checkpoints: Vec<u32>,
        boundaries: Vec<u64>,
        total_bits: usize,
    },
    Cache2 {
        widths: Vec<u8>,
        raws: Vec<u8>,
    },
}

impl Index {
    fn metadata_bytes(&self) -> usize {
        match self {
            Self::Formula { .. } => 0,
            Self::FullUsize { offsets } => offsets.len() * size_of::<usize>(),
            Self::FullU32 { offsets } => offsets.len() * size_of::<u32>(),
            Self::CheckpointU8 { checkpoints, widths, .. } => {
                checkpoints.len() * size_of::<u32>() + widths.len()
            }
            Self::CheckpointW2 { checkpoints, widths, .. } => {
                checkpoints.len() * size_of::<u32>() + widths.byte_len()
            }
            Self::CheckpointW3 { checkpoints, widths, .. } => {
                checkpoints.len() * size_of::<u32>() + widths.byte_len()
            }
            Self::TieredW2 {
                checkpoints,
                locals,
                widths,
                ..
            } => {
                checkpoints.len() * size_of::<u32>() + locals.len() + widths.byte_len()
            }
            Self::BoundarySelect {
                checkpoints,
                boundaries,
                ..
            } => {
                checkpoints.len() * size_of::<u32>() + boundaries.len() * size_of::<u64>()
            }
            Self::Cache2 { widths, raws } => widths.len() + raws.len(),
        }
    }

    fn keeps_packed_payload_hot(&self) -> bool {
        !matches!(self, Self::Cache2 { .. })
    }

    fn query(&self, packed: &PackedBitstream, index: usize) -> (u8, u8, u64) {
        match self {
            Self::Formula { width } => {
                let offset = index * *width as usize;
                (*width, read_word(packed, offset, *width), 0)
            }
            Self::FullUsize { offsets } => {
                let start = offsets[index];
                let width = (offsets[index + 1] - start) as u8;
                (width, read_word(packed, start, width), 0)
            }
            Self::FullU32 { offsets } => {
                let start = offsets[index] as usize;
                let width = (offsets[index + 1] - offsets[index]) as u8;
                (width, read_word(packed, start, width), 0)
            }
            Self::CheckpointU8 { k, checkpoints, widths } => {
                let block_start = (index / *k) * *k;
                let mut offset = checkpoints[index / *k] as usize;
                let mut steps = 0u64;
                for &width in &widths[block_start..index] {
                    offset += width as usize;
                    steps += 1;
                }
                let width = widths[index];
                steps += 1;
                (width, read_word(packed, offset, width), steps)
            }
            Self::CheckpointW2 { k, checkpoints, widths } => {
                let block_start = (index / *k) * *k;
                let mut offset = checkpoints[index / *k] as usize;
                let mut steps = 0u64;
                for position in block_start..index {
                    offset += packed_width2(widths, position) as usize;
                    steps += 1;
                }
                let width = packed_width2(widths, index);
                steps += 1;
                (width, read_word(packed, offset, width), steps)
            }
            Self::CheckpointW3 { k, checkpoints, widths } => {
                let block_start = (index / *k) * *k;
                let mut offset = checkpoints[index / *k] as usize;
                let mut steps = 0u64;
                for position in block_start..index {
                    offset += packed_width(widths, position) as usize;
                    steps += 1;
                }
                let width = packed_width(widths, index);
                steps += 1;
                (width, read_word(packed, offset, width), steps)
            }
            Self::TieredW2 {
                k,
                checkpoints,
                locals,
                widths,
            } => {
                let offset = checkpoints[index / *k] as usize + locals[index] as usize;
                let width = packed_width2(widths, index);
                (width, read_word(packed, offset, width), 0)
            }
            Self::BoundarySelect {
                k,
                checkpoints,
                boundaries,
                total_bits,
            } => {
                let (start, select_probes) =
                    select_boundary(boundaries, checkpoints, *k, index);
                let (end, next_probes) = next_boundary(boundaries, start, *total_bits);
                let width = u8::try_from(end - start).expect("bounded width");
                (
                    width,
                    read_word(packed, start, width),
                    select_probes + next_probes,
                )
            }
            Self::Cache2 { widths, raws } => (widths[index], raws[index], 0),
        }
    }
}

fn parse_checkpoint(candidate: &str, prefix: &str) -> Option<usize> {
    candidate
        .strip_prefix(prefix)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|&k| k > 0)
}

fn build_index(candidate: &str, case: &str, words: &[Word], packed: &PackedBitstream) -> Option<Index> {
    if candidate == "formula" {
        return fixed_width(case).map(|width| Index::Formula { width });
    }
    if candidate == "usize" {
        return Some(Index::FullUsize {
            offsets: full_usize_offsets(words),
        });
    }
    if candidate == "u32" {
        return Some(Index::FullU32 {
            offsets: full_u32_offsets(words),
        });
    }
    if let Some(k) = parse_checkpoint(candidate, "cp8-") {
        return Some(Index::CheckpointU8 {
            k,
            checkpoints: checkpoint_offsets(words, k),
            widths: words.iter().map(|word| word.width).collect(),
        });
    }
    if let Some(k) = parse_checkpoint(candidate, "cp2-") {
        return Some(Index::CheckpointW2 {
            k,
            checkpoints: checkpoint_offsets(words, k),
            widths: packed_width_stream2(words)?,
        });
    }
    if let Some(k) = parse_checkpoint(candidate, "cp3-") {
        return Some(Index::CheckpointW3 {
            k,
            checkpoints: checkpoint_offsets(words, k),
            widths: packed_width_stream(words),
        });
    }
    if let Some(k) = parse_checkpoint(candidate, "t2-") {
        if case != "d1234" || k > 64 {
            return None;
        }
        return Some(Index::TieredW2 {
            k,
            checkpoints: checkpoint_offsets(words, k),
            locals: block_local_offsets_u8(words, k)?,
            widths: packed_width_stream2(words)?,
        });
    }
    if let Some(k) = parse_checkpoint(candidate, "sel-") {
        let (boundaries, checkpoints) = boundary_bitvector(words, k);
        return Some(Index::BoundarySelect {
            k,
            checkpoints,
            boundaries,
            total_bits: total_bits(words),
        });
    }
    if candidate == "cache2" {
        let mut widths = Vec::with_capacity(words.len());
        let mut raws = Vec::with_capacity(words.len());
        let mut offset = 0usize;
        for word in words {
            widths.push(word.width);
            raws.push(read_word(packed, offset, word.width));
            offset += word.width as usize;
        }
        return Some(Index::Cache2 { widths, raws });
    }
    None
}

fn query_index(index: &Index, packed: &PackedBitstream, queries: &[usize], repeats: usize) -> (u64, u64) {
    let mut checksum = 0u64;
    let mut steps = 0u64;
    for rep in 0..repeats {
        for &position in queries {
            let (width, raw, local_steps) = index.query(packed, position);
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(raw) as u64)
                .wrapping_add((width as u64) << 8)
                .wrapping_add(rep as u64 & 1);
            steps = steps.wrapping_add(local_steps);
        }
    }
    (black_box(checksum), black_box(steps))
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let case = args.next().unwrap_or_else(|| "d1234".to_owned());
    let candidate = args.next().unwrap_or_else(|| "u32".to_owned());
    let mode = args.next().unwrap_or_else(|| "prepare".to_owned());
    let count = args.next().and_then(|s| s.parse().ok()).unwrap_or(65_536usize);
    let queries = args.next().and_then(|s| s.parse().ok()).unwrap_or(4_096usize);
    let repeats = args.next().and_then(|s| s.parse().ok()).unwrap_or(1usize);

    if count == 0 || queries == 0 || repeats == 0 {
        eprintln!("count, queries and repeats must be positive");
        return ExitCode::from(2);
    }

    let Some(words) = build_words(&case, count) else {
        eprintln!("unknown case: {case}");
        return ExitCode::from(2);
    };
    let packed = pack_words(&words);
    let query_positions = build_queries(count, queries);
    let semantic_bits = total_bits(&words);
    let packed_bytes = packed.byte_len();

    if mode == "prepare-payload" {
        black_box((&packed, &query_positions));
        println!(
            "case={case} candidate={candidate} mode={mode} count={count} queries={queries} repeats={repeats} semantic_bits={semantic_bits} packed_bytes={packed_bytes} metadata_bytes=0 active_bytes={packed_bytes} checksum={} local_steps=0",
            packed.bit_len()
        );
        return ExitCode::SUCCESS;
    }

    let Some(index) = build_index(&candidate, &case, &words, &packed) else {
        eprintln!("candidate {candidate} is invalid for case {case}");
        return ExitCode::from(2);
    };
    let metadata_bytes = index.metadata_bytes();
    let active_bytes = metadata_bytes
        + if index.keeps_packed_payload_hot() {
            packed_bytes
        } else {
            0
        };

    let (checksum, local_steps) = match mode.as_str() {
        "prepare" => {
            black_box((&index, &packed, &query_positions));
            (metadata_bytes as u64, 0u64)
        }
        "query" => query_index(&index, &packed, &query_positions, repeats),
        _ => {
            eprintln!("unknown mode: {mode}");
            return ExitCode::from(2);
        }
    };

    println!(
        "case={case} candidate={candidate} mode={mode} count={count} queries={queries} repeats={repeats} semantic_bits={semantic_bits} packed_bytes={packed_bytes} metadata_bytes={metadata_bytes} active_bytes={active_bytes} checksum={checksum} local_steps={local_steps}"
    );
    ExitCode::SUCCESS
}
