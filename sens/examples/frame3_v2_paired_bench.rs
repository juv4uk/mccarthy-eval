//! SHA-bound randomized-lane physical T5 / S3-v1 / S3-v2 benchmark.
//!
//! - The SAME exact typed words are decoded/verified in every lane beforehand.
//! - File sizes include the full encoded bytes, including S3 headers and CRC.
//! - Measured wall time is warm in-process encode/decode, not execution.
//! - Each 6-lane order is shuffled deterministically per sample.
//! - Raw paired samples are saved to FRAME3_BENCH_RAW if set.
//! - No assumption about optimality across *all* codecs.
//!
//! cargo run --release --locked -p sens --example frame3_v2_paired_bench
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

use sens::{
    decode_ternary_words, encode_ternary_words, parse_binary_source_words, BinarySourceWord,
};
use sens::frame3_grammar::{encode_body_at_known_length, ExactWord};
use sens::universal_frame3::{decode, decode_v2, encode, encode_v2};

const SAMPLES: usize = 21;
const REPETITIONS: usize = 12;
const LANES: [&str; 6] = [
    "t5-encode", "s3-v1-encode", "s3-v2-encode",
    "t5-decode", "s3-v1-decode", "s3-v2-decode",
];

fn tick(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn choose_words(source: &str) -> Vec<BinarySourceWord> {
    parse_binary_source_words(source)
        .expect("corpus must be syntactically valid exact source words")
        .into_iter()
        .map(|token| token.word)
        .collect()
}

fn lane_call(
    lane: usize,
    words: &[BinarySourceWord],
    t5: &[u8],
    v1: &[u8],
    v2: &[u8],
) {
    match lane {
        0 => { black_box(encode_ternary_words(black_box(words)).unwrap()); }
        1 => { black_box(encode(black_box(words)).unwrap()); }
        2 => { black_box(encode_v2(black_box(words)).unwrap()); }
        3 => { black_box(decode_ternary_words(black_box(t5)).unwrap()); }
        4 => { black_box(decode(black_box(v1)).unwrap()); }
        5 => { black_box(decode_v2(black_box(v2)).unwrap()); }
        _ => unreachable!(),
    }
}

fn bench_case(index: usize, label: &str, source: &str, raw: &mut String) {
    let words = choose_words(source);
    let t5 = encode_ternary_words(&words).expect("T5 corpus admission");
    let v1 = encode(&words).expect("S3/v1 corpus admission");
    let v2 = encode_v2(&words).expect("S3/v2 corpus admission");
    assert_eq!(decode_ternary_words(&t5).unwrap(), words);
    assert_eq!(decode(&v1).unwrap(), words);
    assert_eq!(decode_v2(&v2).unwrap(), words);
    assert_eq!(encode_v2(&decode_v2(&v2).unwrap()).unwrap(), v2);

    // Separately measured lossless grammar-only body. This number is NOT
    // a standalone physical file and must not be compared with whole-file T5.
    let exact = words.iter().copied()
        .map(ExactWord::from_binary_source)
        .collect::<Vec<_>>();
    let grammar = encode_body_at_known_length(&exact).ok();
    let (grammar_bits, grammar_body_bytes) = grammar
        .as_ref()
        .map_or((-1i32, -1i32), |(bits, body)| (*bits as i32, body.len() as i32));

    // Prewarm all six pipelines before recording any sample.
    for lane in 0..LANES.len() {
        lane_call(lane, &words, &t5, &v1, &v2);
    }
    let mut samples: [Vec<u128>; 6] = std::array::from_fn(|_| Vec::with_capacity(SAMPLES));
    let mut rng = 0x9e37_79b9_7f4a_7c15u64 ^ ((index as u64 + 1) << 16);
    for round in 0..SAMPLES {
        let mut order = [0usize, 1, 2, 3, 4, 5];
        for i in (1..order.len()).rev() {
            let j = (tick(&mut rng) % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        for (position, lane) in order.into_iter().enumerate() {
            let start = Instant::now();
            for _ in 0..REPETITIONS {
                lane_call(lane, &words, &t5, &v1, &v2);
            }
            let total_ns = start.elapsed().as_nanos();
            let per_call_ns = total_ns / REPETITIONS as u128;
            samples[lane].push(per_call_ns);
            writeln!(
                raw,
                "{label}\t{round}\t{position}\t{}\t{total_ns}\t{per_call_ns}",
                LANES[lane]
            ).unwrap();
        }
    }
    let mut med = [0u128; 6];
    let mut p95 = [0u128; 6];
    for lane in 0..LANES.len() {
        samples[lane].sort_unstable();
        med[lane] = samples[lane][SAMPLES / 2];
        p95[lane] = samples[lane][(SAMPLES * 95).div_ceil(100) - 1];
    }
    println!(
        "{label}\t{}\t{}\t{}\t{}\t{grammar_bits}\t{grammar_body_bytes}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        words.len(), t5.len(), v1.len(), v2.len(),
        med[0], p95[0], med[1], p95[1], med[2], p95[2],
        med[3], p95[3], med[4], p95[4], med[5], p95[5]
    );
}

fn list_of(w: &str, count: usize) -> String {
    let mut source = String::from("10 ");
    for _ in 0..count {
        source.push_str(w);
        source.push(' ');
    }
    source.push_str("01");
    source
}

fn mixed_w9(count: usize) -> String {
    let mut source = String::from("10 ");
    let mut seed = 0x1234_abcd_u64;
    for _ in 0..count {
        let value = (tick(&mut seed) & 511) as usize;
        write!(source, "{value:09b} ").unwrap();
    }
    source.push_str("01");
    source
}

fn main() {
    let mut raw = String::from("case\tround\tlane_position\tlane\tbatch_total_ns\tns_per_call\n");
    println!("# sha\t{}", std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local-unpinned".into()));
    println!("# methodology\tSHA-bound release; true full-file bytes for T5/S3v1/S3v2, grammar-only bytes separate (not physical); {} samples x {} calls; deterministic shuffled six-lane order; same process and corpus; no execution; p50 and nearest-rank p95; CPU contention not eliminated", SAMPLES, REPETITIONS);
    println!("case\twords\tt5_bytes\tv1_bytes\tv2_bytes\tgrammar_inner_bits\tgrammar_body_bytes_only\tt5_encode_p50_ns\tt5_encode_p95_ns\tv1_encode_p50_ns\tv1_encode_p95_ns\tv2_encode_p50_ns\tv2_encode_p95_ns\tt5_decode_p50_ns\tt5_decode_p95_ns\tv1_decode_p50_ns\tv1_decode_p95_ns\tv2_decode_p50_ns\tv2_decode_p95_ns");

    for (i, (label, source)) in [
        ("empty-list", "10 01".to_owned()),
        ("one-bit-list", "10 0 01".to_owned()),
        ("quote", "10 001 000 01".to_owned()),
        ("nested-quote-data", "10 001 10 10 010 01 01 01".to_owned()),
        ("dotted", "10 0 11 1 01".to_owned()),
        ("separator-exact", "10 001 00 000 01".to_owned()),
        ("many-roots", "10 0 01 10 1 01".to_owned()),
        ("atom-only", "000000001".to_owned()),
        ("w1-repeat-80", list_of("0", 80)),
        ("w3-repeat-80", list_of("000", 80)),
        ("w9-repeat-80", list_of("000000001", 80)),
        ("w9-repeat-512", list_of("000000001", 512)),
        ("w9-mixed-128", mixed_w9(128)),
        ("w9-mixed-512", mixed_w9(512)),
    ].into_iter().enumerate() {
        bench_case(i, label, &source, &mut raw);
    }

    if let Ok(path) = std::env::var("FRAME3_BENCH_RAW") {
        std::fs::write(path, raw).expect("raw sample artifact MUST be saved");
    } else {
        eprintln!("# raw per-sample output omitted: set FRAME3_BENCH_RAW");
    }
}
