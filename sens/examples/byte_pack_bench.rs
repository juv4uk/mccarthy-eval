//! #2190 dense-bit packing mechanism benchmark.
//!
//! Research-only: compares storage/transport mechanics over already-typed
//! exact-width words. It does not assign semantic meaning to any bit pattern.

use sens::{
    append_binary_source_word, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9,
    BinarySourceWord, BitPacker, PackedBitstream,
};
use std::{env, hint::black_box, process::ExitCode};

fn word(workload: &str, i: usize) -> BinarySourceWord {
    match workload {
        "d1" => BinarySourceWord::W1(Bit1::new((i & 1) as u8).unwrap()),
        "d2" => BinarySourceWord::W2(Bit2::new((i & 3) as u8).unwrap()),
        "d3" => BinarySourceWord::W3(Bit3::new((i & 7) as u8).unwrap()),
        "mixed" => match i % 3 {
            0 => BinarySourceWord::W1(Bit1::new((i & 1) as u8).unwrap()),
            1 => BinarySourceWord::W2(Bit2::new((i & 3) as u8).unwrap()),
            _ => BinarySourceWord::W3(Bit3::new((i & 7) as u8).unwrap()),
        },
        "w4" => BinarySourceWord::W4(Bit4::new((i & 15) as u8).unwrap()),
        "w5" => BinarySourceWord::W5(Bit5::new((i & 31) as u8).unwrap()),
        "w6" => BinarySourceWord::W6(Bit6::new((i & 63) as u8).unwrap()),
        "w7" => BinarySourceWord::W7(Bit7::new((i & 127) as u8).unwrap()),
        "w8" => BinarySourceWord::W8(Bit8::new((i & 255) as u8).unwrap()),
        "w9" => BinarySourceWord::W9(Bit9::new((i & 511) as u16).unwrap()),
        _ => panic!("unknown workload"),
    }
}

fn corpus(workload: &str, n: usize) -> Vec<BinarySourceWord> {
    (0..n).map(|i| word(workload, i)).collect()
}

fn pack(words: &[BinarySourceWord]) -> (PackedBitstream, Vec<usize>) {
    let total_bits: usize = words.iter().map(|w| w.width()).sum();
    let mut packer = BitPacker::with_capacity_bits(total_bits);
    let mut offsets = Vec::with_capacity(words.len());
    for &w in words {
        offsets.push(append_binary_source_word(&mut packer, w));
    }
    (packer.finish(), offsets)
}

fn read_dynamic(packed: &PackedBitstream, offset: usize, width: usize) -> u16 {
    match width {
        1 => packed.read::<1>(offset).unwrap().packed_bits() as u16,
        2 => packed.read::<2>(offset).unwrap().packed_bits() as u16,
        3 => packed.read::<3>(offset).unwrap().packed_bits() as u16,
        4 => packed.read::<4>(offset).unwrap().packed_bits() as u16,
        5 => packed.read::<5>(offset).unwrap().packed_bits() as u16,
        6 => packed.read::<6>(offset).unwrap().packed_bits() as u16,
        7 => packed.read::<7>(offset).unwrap().packed_bits() as u16,
        8 => packed.read::<8>(offset).unwrap().packed_bits() as u16,
        9 => packed.read_w9(offset).unwrap().packed_bits(),
        _ => unreachable!(),
    }
}

fn verify(words: &[BinarySourceWord]) -> (usize, usize, u64) {
    let (packed, offsets) = pack(words);
    let mut checksum = 0u64;
    for (index, &w) in words.iter().enumerate() {
        let decoded = read_dynamic(&packed, offsets[index], w.width());
        assert_eq!(u16::from(decoded), w.packed_bits());
        checksum = checksum
            .wrapping_add(decoded as u64)
            .wrapping_add(w.width() as u64);
    }
    (packed.bit_len(), packed.byte_len(), checksum)
}

fn unpacked_scan(words: &[BinarySourceWord], reps: usize) -> u64 {
    let mut checksum = 0u64;
    for _ in 0..reps {
        for &w in words {
            let w = black_box(w);
            checksum = checksum
                .wrapping_add(w.packed_bits() as u64)
                .wrapping_add(w.width() as u64);
        }
    }
    black_box(checksum)
}

fn pack_scan(words: &[BinarySourceWord], reps: usize) -> u64 {
    let mut checksum = 0u64;
    for _ in 0..reps {
        let mut packer = BitPacker::with_capacity_bits(
            words.iter().map(|w| w.width()).sum(),
        );
        for &w in words {
            checksum = checksum.wrapping_add(append_binary_source_word(&mut packer, w) as u64);
        }
        let packed = packer.finish();
        checksum = checksum
            .wrapping_add(packed.bit_len() as u64)
            .wrapping_add(packed.byte_len() as u64);
        black_box(&packed);
    }
    black_box(checksum)
}

fn packed_decode(words: &[BinarySourceWord], reps: usize) -> u64 {
    let (packed, offsets) = pack(words);
    let mut checksum = 0u64;
    for _ in 0..reps {
        for (index, &w) in words.iter().enumerate() {
            let value = read_dynamic(black_box(&packed), offsets[index], w.width());
            checksum = checksum
                .wrapping_add(value as u64)
                .wrapping_add(w.width() as u64);
        }
    }
    black_box(checksum)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: byte_pack_bench MODE WORKLOAD N REPS");
        return ExitCode::from(2);
    }
    let mode = &args[1];
    let workload = &args[2];
    let n: usize = args[3].parse().expect("N");
    let reps: usize = args[4].parse().expect("REPS");
    assert!(n > 0 && reps > 0);

    let words = corpus(workload, n);
    let semantic_bits: usize = words.iter().map(|w| w.width()).sum();
    let minimum_bytes = semantic_bits.div_ceil(8);

    if mode == "verify" {
        let (packed_bits, packed_bytes, parity_checksum) = verify(&words);
        assert_eq!(packed_bits, semantic_bits);
        assert_eq!(packed_bytes, minimum_bytes);
        println!(
            "PACK_BENCH\tmode=verify\tworkload={workload}\tn={n}\treps={reps}\tsemantic_bits={semantic_bits}\tphysical_bytes={packed_bytes}\tchecksum={parity_checksum}"
        );
        return ExitCode::SUCCESS;
    }

    let checksum = match mode.as_str() {
        "base" => black_box(words.len() as u64),
        "unpacked" => unpacked_scan(&words, reps),
        "pack" => pack_scan(&words, reps),
        "decode" => packed_decode(&words, reps),
        _ => {
            eprintln!("unknown mode: {mode}");
            return ExitCode::from(2);
        }
    };

    println!(
        "PACK_BENCH\tmode={mode}\tworkload={workload}\tn={n}\treps={reps}\tsemantic_bits={semantic_bits}\tphysical_bytes={minimum_bytes}\tchecksum={checksum}"
    );
    ExitCode::SUCCESS
}
