//! #2190: density vs CPU cost for production exact-width packing.
//!
//! This benchmark is mechanical only. Width schedules are supplied out of band
//! to both the byte-per-word and packed lanes, so no framing/semantic authority
//! is hidden in either side.

use sens::{Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, BitPacker, PackedBitstream};
use std::{env, hint::black_box, process::ExitCode};

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

fn width_at(case: &str, index: usize) -> Option<u8> {
    let width = match case {
        "w1" => 1,
        "w2" => 2,
        "w3" => 3,
        "w4" => 4,
        "w5" => 5,
        "w6" => 6,
        "w7" => 7,
        "w8" => 8,
        "d34" => [3, 4][index % 2],
        "d1234" => [2, 3, 2, 4, 2, 3, 1, 4, 2, 3, 4, 3][index % 12],
        "mixed18" => (index % 8 + 1) as u8,
        _ => return None,
    };
    Some(width)
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
        1 => {
            packer.push(Bit1::new(word.raw).unwrap());
        }
        2 => {
            packer.push(Bit2::new(word.raw).unwrap());
        }
        3 => {
            packer.push(Bit3::new(word.raw).unwrap());
        }
        4 => {
            packer.push(Bit4::new(word.raw).unwrap());
        }
        5 => {
            packer.push(Bit5::new(word.raw).unwrap());
        }
        6 => {
            packer.push(Bit6::new(word.raw).unwrap());
        }
        7 => {
            packer.push(Bit7::new(word.raw).unwrap());
        }
        8 => {
            packer.push(Bit8::new(word.raw).unwrap());
        }
        _ => unreachable!(),
    }
}

fn pack_words(words: &[Word]) -> PackedBitstream {
    let bits = words.iter().map(|word| word.width as usize).sum();
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

fn decode_cache(words: &[Word], packed: &PackedBitstream) -> Vec<u8> {
    let mut cache = Vec::with_capacity(words.len());
    let mut offset = 0usize;
    for word in words {
        cache.push(read_word(packed, offset, word.width));
        offset += word.width as usize;
    }
    cache
}

fn materialize_unpacked_bytes(words: &[Word]) -> Vec<u8> {
    words.iter().map(|word| word.raw).collect()
}

fn scan_unpacked(bytes: &[u8], repeats: usize) -> u64 {
    let mut checksum = 0u64;
    for rep in 0..repeats {
        for &raw in bytes {
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(raw) as u64)
                .wrapping_add(rep as u64 & 1);
        }
    }
    black_box(checksum)
}

fn scan_packed(words: &[Word], packed: &PackedBitstream, repeats: usize) -> u64 {
    let mut checksum = 0u64;
    for rep in 0..repeats {
        let mut offset = 0usize;
        for word in words {
            let raw = read_word(packed, offset, word.width);
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(raw) as u64)
                .wrapping_add(rep as u64 & 1);
            offset += word.width as usize;
        }
    }
    black_box(checksum)
}

fn scan_cache(cache: &[u8], repeats: usize) -> u64 {
    let mut checksum = 0u64;
    for rep in 0..repeats {
        for &raw in cache {
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(raw) as u64)
                .wrapping_add(rep as u64 & 1);
        }
    }
    black_box(checksum)
}

fn pack_words_with_offsets(words: &[Word]) -> (PackedBitstream, Vec<usize>) {
    let bits = words.iter().map(|word| word.width as usize).sum();
    let mut packer = BitPacker::with_capacity_bits(bits);
    let mut offsets = Vec::with_capacity(words.len());
    for &word in words {
        offsets.push(packer.bit_len());
        push_word(&mut packer, word);
    }
    (packer.finish(), offsets)
}

fn build_random_indices(count: usize, accesses: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(accesses);
    let mut state = 0x243f_6a88_u32 ^ count as u32 ^ accesses as u32;
    for _ in 0..accesses {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push((state as usize) % count);
    }
    out
}

fn random_unpacked(bytes: &[u8], indices: &[usize], repeats: usize) -> u64 {
    let mut checksum = 0u64;
    for rep in 0..repeats {
        for &index in indices {
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(bytes[index]) as u64)
                .wrapping_add(rep as u64 & 1);
        }
    }
    black_box(checksum)
}

fn random_packed(
    words: &[Word],
    packed: &PackedBitstream,
    offsets: &[usize],
    fixed_width: Option<u8>,
    indices: &[usize],
    repeats: usize,
) -> u64 {
    let mut checksum = 0u64;
    for rep in 0..repeats {
        for &index in indices {
            let offset = match fixed_width {
                Some(width) => index * width as usize,
                None => offsets[index],
            };
            let raw = read_word(packed, offset, words[index].width);
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(raw) as u64)
                .wrapping_add(rep as u64 & 1);
        }
    }
    black_box(checksum)
}

fn random_cache(cache: &[u8], indices: &[usize], repeats: usize) -> u64 {
    let mut checksum = 0u64;
    for rep in 0..repeats {
        for &index in indices {
            checksum = checksum
                .wrapping_mul(33)
                .wrapping_add(black_box(cache[index]) as u64)
                .wrapping_add(rep as u64 & 1);
        }
    }
    black_box(checksum)
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let case = args.next().unwrap_or_else(|| "d1234".to_owned());
    let mode = args.next().unwrap_or_else(|| "prepare-unpacked".to_owned());
    let count = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(65_536usize);
    let repeats = args.next().and_then(|s| s.parse().ok()).unwrap_or(8usize);
    let random_accesses = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| count.min(65_536));

    if count == 0 || repeats == 0 || random_accesses == 0 {
        eprintln!("count, repeats and random_accesses must be positive");
        return ExitCode::from(2);
    }

    let Some(words) = build_words(&case, count) else {
        eprintln!("unknown case: {case}");
        return ExitCode::from(2);
    };
    let semantic_bits: usize = words.iter().map(|word| word.width as usize).sum();
    let unpacked_bytes = words.len();

    let (checksum, packed_bytes, cache_bytes, offset_index_bytes, access_index_bytes) = match mode.as_str() {
        "prepare-logical" => {
            black_box(&words);
            (words.len() as u64, 0usize, 0usize, 0usize, 0usize)
        }
        "prepare-unpacked" => {
            let bytes = materialize_unpacked_bytes(&words);
            let len = bytes.len();
            black_box(&bytes);
            (len as u64, 0usize, len, 0usize, 0usize)
        }
        "prepare-packed" => {
            let packed = pack_words(&words);
            let bytes = packed.byte_len();
            black_box(&packed);
            (packed.bit_len() as u64, bytes, 0, 0usize, 0usize)
        }
        "prepare-cached" => {
            let packed = pack_words(&words);
            let bytes = packed.byte_len();
            let cache = decode_cache(&words, &packed);
            let cache_len = cache.len();
            black_box(&cache);
            (cache_len as u64, bytes, cache_len, 0usize, 0usize)
        }
        "scan-unpacked" => {
            let bytes = materialize_unpacked_bytes(&words);
            let len = bytes.len();
            let checksum = scan_unpacked(&bytes, repeats);
            (checksum, 0, len, 0usize, 0usize)
        }
        "scan-packed" => {
            let packed = pack_words(&words);
            let bytes = packed.byte_len();
            let checksum = scan_packed(&words, &packed, repeats);
            (checksum, bytes, 0, 0usize, 0usize)
        }
        "scan-cached" => {
            let packed = pack_words(&words);
            let bytes = packed.byte_len();
            let cache = decode_cache(&words, &packed);
            let cache_len = cache.len();
            let checksum = scan_cache(&cache, repeats);
            (checksum, bytes, cache_len, 0usize, 0usize)
        }
        "prepare-random-unpacked" => {
            let bytes = materialize_unpacked_bytes(&words);
            let indices = build_random_indices(count, random_accesses);
            let index_bytes = indices.len() * std::mem::size_of::<usize>();
            let len = bytes.len();
            black_box((&bytes, &indices));
            (len as u64, 0usize, len, 0usize, index_bytes)
        }
        "prepare-random-packed" => {
            let (packed, offsets) = match fixed_width(&case) {
                Some(_) => (pack_words(&words), Vec::new()),
                None => pack_words_with_offsets(&words),
            };
            let indices = build_random_indices(count, random_accesses);
            let packed_bytes = packed.byte_len();
            let offset_bytes = offsets.len() * std::mem::size_of::<usize>();
            let index_bytes = indices.len() * std::mem::size_of::<usize>();
            black_box((&packed, &offsets, &indices));
            (
                packed.bit_len() as u64,
                packed_bytes,
                0usize,
                offset_bytes,
                index_bytes,
            )
        }
        "prepare-random-cached" => {
            let packed = pack_words(&words);
            let cache = decode_cache(&words, &packed);
            let indices = build_random_indices(count, random_accesses);
            let packed_bytes = packed.byte_len();
            let cache_bytes = cache.len();
            let index_bytes = indices.len() * std::mem::size_of::<usize>();
            black_box((&cache, &indices));
            (
                cache_bytes as u64,
                packed_bytes,
                cache_bytes,
                0usize,
                index_bytes,
            )
        }
        "random-unpacked" => {
            let bytes = materialize_unpacked_bytes(&words);
            let indices = build_random_indices(count, random_accesses);
            let index_bytes = indices.len() * std::mem::size_of::<usize>();
            let checksum = random_unpacked(&bytes, &indices, repeats);
            (checksum, 0usize, bytes.len(), 0usize, index_bytes)
        }
        "random-packed" => {
            let fixed = fixed_width(&case);
            let (packed, offsets) = match fixed {
                Some(_) => (pack_words(&words), Vec::new()),
                None => pack_words_with_offsets(&words),
            };
            let indices = build_random_indices(count, random_accesses);
            let packed_bytes = packed.byte_len();
            let offset_bytes = offsets.len() * std::mem::size_of::<usize>();
            let index_bytes = indices.len() * std::mem::size_of::<usize>();
            let checksum = random_packed(&words, &packed, &offsets, fixed, &indices, repeats);
            (checksum, packed_bytes, 0usize, offset_bytes, index_bytes)
        }
        "random-cached" => {
            let packed = pack_words(&words);
            let cache = decode_cache(&words, &packed);
            let indices = build_random_indices(count, random_accesses);
            let packed_bytes = packed.byte_len();
            let cache_bytes = cache.len();
            let index_bytes = indices.len() * std::mem::size_of::<usize>();
            let checksum = random_cache(&cache, &indices, repeats);
            (checksum, packed_bytes, cache_bytes, 0usize, index_bytes)
        }
        _ => {
            eprintln!("unknown mode: {mode}");
            return ExitCode::from(2);
        }
    };

    println!(
        "case={case} mode={mode} count={count} repeats={repeats} random_accesses={random_accesses} semantic_bits={semantic_bits} unpacked_bytes={unpacked_bytes} packed_bytes={packed_bytes} cache_bytes={cache_bytes} offset_index_bytes={offset_index_bytes} access_index_bytes={access_index_bytes} checksum={checksum}"
    );
    ExitCode::SUCCESS
}
