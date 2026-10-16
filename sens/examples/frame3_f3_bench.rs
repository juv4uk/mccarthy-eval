//! Порівняння оригінальної рамки-3 (F3, .senc) з T5/S3v1/S3v2/S3v3.
//! Рамка-3 приймає лише одну зовнішню D2-рамку `10 … 01` без сусідніх 22.
//! Фікстури з кількома верхньорівневими формами або атомами пропускаються.
use std::hint::black_box;
use std::time::Instant;

use sens::{
    decode_ternary_words, encode_ternary_words, parse_binary_source_words, BinarySourceWord,
};
use sens::universal_frame3::{decode, decode_v2, encode, encode_v2};
use sens::frame3_domain::{encode as encode_v3, decode as decode_v3, Word};

#[path = "../../../research/framed3/rust_codec/senc.rs"]
mod senc;

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
    let v3_words: Vec<Word> = words.iter().map(|w| Word::Core(w.clone())).collect();
    let v3 = encode_v3(&v3_words).expect("Frame3/v3 encoded corpus");
    assert_eq!(decode_ternary_words(&t5).unwrap(), words);
    assert_eq!(decode(&v1).unwrap(), words);
    assert_eq!(decode_v2(&v2).unwrap(), words);
    assert_eq!(decode_v3(&v3).unwrap(), v3_words);

    let word_strings: Vec<String> = words
        .iter()
        .map(|w| {
            let packed = w.packed_bits();
            let width = w.width();
            (0..width)
                .rev()
                .map(|i| if (packed >> i) & 1 == 1 { '1' } else { '0' })
                .collect()
        })
        .collect();
    let f3 = match senc::frame_encode(&word_strings) {
        Ok(data) => {
            let mut full = vec![senc::FRAME_LABEL];
            full.extend_from_slice(&data);
            full
        }
        Err(_) => {
            println!("{}\t{}\t{}\t{}\t{}\t{}\tNA", name, words.len(), t5.len(), v1.len(), v2.len(), v3.len());
            return;
        }
    };
    let f3_decoded = match senc::decode_senc(&f3) {
        Ok(decoded) => decoded,
        Err(senc::SencError::NotCanonical) => {
            eprintln!("# F3_REJECTED_NONCANONICAL\t{name}\tcurrent canonical form law");
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\tNA",
                name, words.len(), t5.len(), v1.len(), v2.len(), v3.len()
            );
            return;
        }
        Err(error) => panic!("F3 decoder rejected admissible source unexpectedly: {error:?}"),
    };
    assert_eq!(f3_decoded, word_strings);

    let (t5_en_p50, t5_en_p95) = timings(|| {
        black_box(encode_ternary_words(black_box(&words)).unwrap());
    });
    let (v1_en_p50, v1_en_p95) = timings(|| {
        black_box(encode(black_box(&words)).unwrap());
    });
    let (v2_en_p50, v2_en_p95) = timings(|| {
        black_box(encode_v2(black_box(&words)).unwrap());
    });
    let (v3_en_p50, v3_en_p95) = timings(|| {
        black_box(encode_v3(black_box(&v3_words)).unwrap());
    });
    let (f3_en_p50, f3_en_p95) = timings(|| {
        let body = senc::frame_encode(black_box(&word_strings)).unwrap();
        let mut full = vec![senc::FRAME_LABEL];
        full.extend_from_slice(&body);
        black_box(full);
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
    let (v3_de_p50, v3_de_p95) = timings(|| {
        black_box(decode_v3(black_box(&v3)).unwrap());
    });
    let (f3_de_p50, f3_de_p95) = timings(|| {
        black_box(senc::decode_senc(black_box(&f3)).unwrap());
    });
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        name,
        words.len(),
        t5.len(), v1.len(), v2.len(), v3.len(), f3.len(),
        t5_en_p50, t5_en_p95,
        v1_en_p50, v1_en_p95,
        v2_en_p50, v2_en_p95,
        v3_en_p50, v3_en_p95,
        f3_en_p50, f3_en_p95,
        t5_de_p50, t5_de_p95,
        v1_de_p50, v1_de_p95,
        v2_de_p50, v2_de_p95,
        v3_de_p50, v3_de_p95,
        f3_de_p50, f3_de_p95
    );
}

fn main() {
    println!("# SHA\t{}", std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local-unpinned".into()));
    println!("# runner OS\t{}", std::env::var("BENCH_OS").unwrap_or_else(|_| "unknown".into()));
    println!("# rustc\t{}", std::env::var("BENCH_RUSTC").unwrap_or_else(|_| "unknown".into()));
    println!("# CPU\t{}", std::env::var("BENCH_CPU").unwrap_or_else(|_| "unknown".into()));
    println!("# scope\trelease; same process, same sample/corpus; {} samples x {} repetitions; ns/one call, p50/p95; T5/Frame3-v1/v2/v3/F3 are different physical formats; do not equate this with language execution; hosted CPUs vary", SAMPLES, REPS);
    println!("case\twords\tt5_bytes\tv1_bytes\tv2_bytes\tv3_bytes\tf3_bytes\tt5_encode_p50_ns\tt5_encode_p95_ns\tv1_encode_p50_ns\tv1_encode_p95_ns\tv2_encode_p50_ns\tv2_encode_p95_ns\tv3_encode_p50_ns\tv3_encode_p95_ns\tf3_encode_p50_ns\tf3_encode_p95_ns\tt5_decode_p50_ns\tt5_decode_p95_ns\tv1_decode_p50_ns\tv1_decode_p95_ns\tv2_decode_p50_ns\tv2_decode_p95_ns\tv3_decode_p50_ns\tv3_decode_p95_ns\tf3_decode_p50_ns\tf3_decode_p95_ns");

    for (name, fixture) in [
        ("empty-list", "10 01"),
        ("single-d3-quote", "10 001 000 01"),
        ("nested-quote-data", "10 001 10 10 010 01 01 01"),
        ("dotted-pair", "10 0 11 1 01"),
        ("separator-exact", "10 001 00 000 01"),
        ("mixed-width", "10 0111 101010101 00 0 11 1 01"),
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
