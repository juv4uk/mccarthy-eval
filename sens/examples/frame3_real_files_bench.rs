//! Git-tracked real .sens files, physical-byte and roundtrip comparison.
//! Never relabel S3/v1-v3 as original Frame-3; original F3 is imported from
//! research/framed3/rust_codec/senc.rs. No execution of arbitrary repository
//! files, only real Rust canonical reader admission. This is a research bench.
use std::{fs, path::Path, process::Command, time::Instant};
use sens::{
    decode_ternary_words, encode_ternary_words, parse_canonical_word_sequence,
    PhysicalT5Program,
};
#[path = "../../../research/framed3/rust_codec/senc.rs"]
mod senc;

const SAMPLES: usize = 15;
const INNER: usize = 16;

/// Warm repeated batches reduce sub-microsecond timer quantization;
/// p50/p95 are per-operation wall times on GitHub-hosted CPU.
fn time_ns(mut op: impl FnMut()) -> (f64, f64) {
    for _ in 0..INNER { op(); }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..INNER { op(); }
        let elapsed = start.elapsed().as_nanos();
        samples.push(elapsed as f64 / INNER as f64);
    }
    samples.sort_by(f64::total_cmp);
    (samples[SAMPLES / 2], samples[(SAMPLES * 95).div_ceil(100) - 1])
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let git = Command::new("git").args(["ls-files", "-z", "--", "*.sens"])
        .output()?;
    if !git.status.success() {
        return Err("git tracked-file inventory failed".into());
    }
    let mut paths = git.stdout.split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    if paths.is_empty() {
        return Err("no tracked physical .sens files; no benchmark evidence".into());
    }
    println!("# corpus=real git-tracked physical .sens, no synthetic repetition");
    println!("# F3=original research carrier with 1-byte profile label included");
    println!("# timings=warm buffer-only Rust encode/decode, {SAMPLES} samples of {INNER} operations each; p50/p95 ns/op; per-file unpaired");
    println!("# this benchmark checks Rust D2 admission, NOT evaluation of arbitrary files");
    println!("file\tstatus\tt5_bytes\tf3_total_bytes\tt5_encode_p50_ns\tt5_encode_p95_ns\tf3_encode_p50_ns\tf3_encode_p95_ns\tt5_decode_p50_ns\tt5_decode_p95_ns\tf3_decode_p50_ns\tf3_decode_p95_ns");
    let mut scanned = 0usize;
    let mut admitted = 0usize;
    let mut f3_supported = 0usize;
    let mut t5_total = 0usize;
    let mut f3_total = 0usize;
    for path in paths {
        scanned += 1;
        let raw = fs::read(Path::new(&path))?;
        let Ok(physical) = PhysicalT5Program::decode(&raw) else {
            println!("{path}\tREJECTED_RUST_T5_READER\t{}\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA\tNA",raw.len());
            continue;
        };
        let words = decode_ternary_words(&raw).map_err(|e| format!("T5 decode after admission: {e:?}"))?;
        if encode_ternary_words(&words).map_err(|e| format!("T5 encode: {e:?}"))? != raw {
            return Err(format!("noncanonical physical T5 accepted: {path}").into());
        }
        let parse = parse_canonical_word_sequence(&words);
        if parse.is_err() || physical.form_count() == 0 {
            return Err(format!("Rust T5 admission inconsistent: {path}").into());
        }
        admitted += 1;
        let as_text = words.iter().map(|word| {
            let bits = word.packed_bits();
            format!("{bits:0width$b}",width=word.width())
        }).collect::<Vec<_>>();
        let t5_en = time_ns(|| {
            std::hint::black_box(encode_ternary_words(std::hint::black_box(&words)).unwrap());
        });
        let t5_de = time_ns(|| {
            std::hint::black_box(decode_ternary_words(std::hint::black_box(&raw)).unwrap());
        });
        let Ok(body) = senc::frame_encode(&as_text) else {
            println!("{path}\tF3_UNSUPPORTED\t{}\tNA\t{:.2}\t{:.2}\tNA\tNA\t{:.2}\t{:.2}\tNA\tNA",raw.len(), t5_en.0, t5_en.1, t5_de.0, t5_de.1);
            continue;
        };
        let mut f3 = vec![senc::FRAME_LABEL];
        f3.extend_from_slice(&body);
        match senc::decode_senc(&f3) {
            Ok(restored) if restored == as_text => {}
            Err(senc::SencError::NotCanonical) => {
                println!("{path}\tF3_REJECTED_NONCANONICAL\t{}\tNA\t{:.2}\t{:.2}\tNA\tNA\t{:.2}\t{:.2}\tNA\tNA",raw.len(), t5_en.0, t5_en.1, t5_de.0, t5_de.1);
                continue;
            }
            other => return Err(format!("F3 did not roundtrip exact source: {path}: {other:?}").into()),
        }
        f3_supported += 1;
        t5_total += raw.len();
        f3_total += f3.len();
        let f3_en = time_ns(|| {
            let data = senc::frame_encode(std::hint::black_box(&as_text)).unwrap();
            let mut full = vec![senc::FRAME_LABEL];
            full.extend_from_slice(&data);
            std::hint::black_box(full);
        });
        let f3_de = time_ns(|| {
            std::hint::black_box(senc::decode_senc(std::hint::black_box(&f3)).unwrap());
        });
        println!("{path}\tROUNDTRIP_OK\t{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}",raw.len(), f3.len(), t5_en.0, t5_en.1, f3_en.0, f3_en.1, t5_de.0, t5_de.1, f3_de.0, f3_de.1);
    }
    eprintln!("REAL_FILE_BENCH: tracked={scanned}, Rust-T5-admitted={admitted}, F3-exact-roundtrip={f3_supported}, compared-only-on-F3-supported T5={t5_total}B F3={f3_total}B");
    if admitted == 0 {
        return Err("no canonically admitted real files, no evidence".into());
    }
    // F3 may support none of the corpus; that is a reported result, NOT a green
    // compression success. This step does not ever silently fabricate bytes.
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("REAL_FILE_BENCH_BLOCKED: {e}");
        std::process::exit(1);
    }
}
