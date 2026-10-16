//! Same-run physical file + wall-time comparison: T5 versus S3/v1 versus S3/v2.
//! No semantic speed claims; every lane encodes/decodes identical exact words.
//! Use cargo run --release -p sens --example frame3_v2_bench.
//! TSV stdout is a directly downloadable reproducible CI artifact.
use std::hint::black_box;
use std::time::Instant;

use sens::{
    decode_ternary_words, encode_ternary_words, parse_binary_source_words, BinarySourceWord,
};
use sens::universal_frame3::{decode, decode_v2, encode, encode_v2};

const SAMPLES: usize = 11;
const REPS: usize = 12;

fn timings(mut run: impl FnMut()) -> (u128, u128) {
    let mut values = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..REPS {
            run();
        }
        values.push(start.elapsed().as_nanos() / REPS as u128);
    }
    values.sort_unstable();
    (values[SAMPLES / 2], values[(SAMPLES * 95).div_ceil(100) - 1])
}

fn run_case(name: &str, source: &str) {
    let words: Vec<BinarySourceWord> = parse_binary_source_words(source)
        .expect("exact source word corpus")
        .into_iter()
        .map(|item| item.word)
        .collect();
    let t5 = encode_ternary_words(&words).expect("T5 encoded corpus");
    let v1 = encode(&words).expect("Frame3/v1 encoded corpus");
    let v2 = encode_v2(&words).expect("Frame3/v2 encoded corpus");
    assert_eq!(decode_ternary_words(&t5).unwrap(), words);
    assert_eq!(decode(&v1).unwrap(), words);
    assert_eq!(decode_v2(&v2).unwrap(), words);
    assert_eq!(encode_v2(&decode_v2(&v2).unwrap()).unwrap(), v2);

    let (t5_en_p50, t5_en_p95) = timings(|| {
        black_box(encode_ternary_words(black_box(&words)).unwrap());
    });
    let (v1_en_p50, v1_en_p95) = timings(|| {
        black_box(encode(black_box(&words)).unwrap());
    });
    let (v2_en_p50, v2_en_p95) = timings(|| {
        black_box(encode_v2(black_box(&words)).unwrap());
    });
    let (t5_de_p50, t5_de_p95) = timings(|| {
        black_box(decode_ternary_words(black_box(&t5)).unwrap());
    });
    let (v1_de_p50, v1_de_p95) = timings(|| {
        black_box(decode(black_box(&v1)).unwrap());
    });
    let (v2_de_p50, v2_de_p95) = timings(|| {
        black_box(decode_v2(black_box(&v2)).unwrap());
    });
    println!(
        "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        words.len(),
        t5.len(), v1.len(), v2.len(),
        t5_en_p50, t5_en_p95,
        v1_en_p50, v1_en_p95,
        v2_en_p50, v2_en_p95,
        t5_de_p50, t5_de_p95,
        v1_de_p50, v1_de_p95,
        v2_de_p50, v2_de_p95
    );
}

fn main() {
    println!("# SHA\t{}", std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local-unpinned".into()));
    println!("# scope\trelease; same process, same sample/corpus; {} samples x {} repetitions; ns/one call, p50/p95; T5/Frame3-v1/v2 are different physical formats; do not equate this with language execution; hosted CPUs vary", SAMPLES, REPS);
    println!("case\twords\tt5_bytes\tv1_bytes\tv2_bytes\tt5_encode_p50_ns\tt5_encode_p95_ns\tv1_encode_p50_ns\tv1_encode_p95_ns\tv2_encode_p50_ns\tv2_encode_p95_ns\tt5_decode_p50_ns\tt5_decode_p95_ns\tv1_decode_p50_ns\tv1_decode_p95_ns\tv2_decode_p50_ns\tv2_decode_p95_ns");

    for (name, fixture) in [
        ("empty-list", "10 01"),
        ("single-d3-quote", "10 001 000 01"),
        ("nested-quote-data", "10 001 10 10 010 01 01 01"),
        ("dotted-pair", "10 0 11 1 01"),
        ("separator-exact", "10 001 00 000 01"),
        ("multiple-roots", "10 0 01 10 1 01"),
        ("mixed-width", "10 0111 101010101 00 0 11 1 01"),
        ("top-level-atom", "000000001"),
    ] {
        run_case(name, fixture);
    }

    let mut repeated = String::from("10 ");
    for _ in 0..80 {
        repeated.push_str("000000001 ");
    }
    repeated.push_str("01");
    run_case("repeated-w9-80", &repeated);
}
