//! Exact matched T5 mechanical benchmark baseline for Zig (manual, release).
//! Workload follows research/zig-codecs/t5/bench.zig byte for byte.
//! This is transport benchmarking, NOT a SENS semantic oracle.
use std::hint::black_box;
use std::time::Instant;

use sens::{decode_ternary_words, encode_ternary_words, parse_binary_source_words, BinarySourceWord};

#[global_allocator]
static ALLOCATOR: std::alloc::System = std::alloc::System;

fn fnv64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn trial(
    operation: &str,
    words: &[BinarySourceWord],
    physical: &[u8],
    forms: usize,
    repetitions: usize,
) {
    let begin = Instant::now();
    for _ in 0..repetitions {
        match operation {
            "encode" => {
                black_box(encode_ternary_words(black_box(words)).expect("T5 encode"));
            }
            "decode" => {
                black_box(decode_ternary_words(black_box(physical)).expect("T5 decode"));
            }
            _ => panic!("unsupported benchmark operation"),
        }
    }
    let elapsed = begin.elapsed().as_nanos();
    println!(
        "sample impl=rust forms={} op={} elapsed_ns={} reps={}",
        forms, operation, elapsed, repetitions
    );
}

fn main() {
    let cases = [1usize, 16, 128, 1024];
    let reps = [12000usize, 3000, 600, 100];
    let pattern: Vec<BinarySourceWord> = parse_binary_source_words("10 100 01")
        .expect("typed binary test words")
        .into_iter()
        .map(|token| token.word)
        .collect();
    assert_eq!(pattern.len(), 3);
    for (forms, repetitions) in cases.into_iter().zip(reps) {
        let mut words = Vec::<BinarySourceWord>::with_capacity(forms * pattern.len());
        for _ in 0..forms {
            words.extend_from_slice(&pattern);
        }
        let physical = encode_ternary_words(&words).expect("T5 encode");
        let recovered = decode_ternary_words(&physical).expect("T5 decode");
        assert_eq!(recovered, words, "mechanical T5 parity");
        println!(
            "fixture impl=rust forms={} words={} bytes={} fnv64={:x}",
            forms, words.len(), physical.len(), fnv64(&physical)
        );
        print!("physical impl=rust forms={} hex=", forms);
        for byte in &physical {
            print!("{byte:02x}");
        }
        println!();

        for _ in 0..3 {
            black_box(encode_ternary_words(&words).expect("warm encode"));
            black_box(decode_ternary_words(&physical).expect("warm decode"));
        }
        for _ in 0..5 {
            trial("encode", &words, &physical, forms, repetitions);
            trial("decode", &words, &physical, forms, repetitions);
        }
    }
}
