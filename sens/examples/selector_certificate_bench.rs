//! #2323 selector generation-certificate positive control.
//!
//! Research-only. A certificate reconstructs an exact selector coordinate from
//! one of two admitted D3 roots plus a suffix-law path. It grants no authority
//! to non-selector coordinates and is not a production wire format.

use std::{env, hint::black_box, process::ExitCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Word {
    bits: u16,
    width: u8,
}

#[derive(Clone, Copy, Debug)]
struct Cert {
    payload: u16,
    bit_len: u8,
}

fn encode(root_choice: u8, depth: u8, path: u16) -> Option<Cert> {
    if root_choice > 1 || depth > 5 {
        return None;
    }
    let path_limit = 1u16 << depth;
    if path >= path_limit {
        return None;
    }
    let bit_len = 4 + depth;
    let payload =
        ((root_choice as u16) << (depth + 3))
        | ((depth as u16) << depth)
        | path;
    Some(Cert { payload, bit_len })
}

fn decode(cert: Cert) -> Option<Word> {
    if !(4..=9).contains(&cert.bit_len) {
        return None;
    }
    let depth = cert.bit_len - 4;
    let mask = (1u16 << cert.bit_len) - 1;
    if cert.payload & !mask != 0 {
        return None;
    }

    let encoded_depth = ((cert.payload >> depth) & 0b111) as u8;
    if encoded_depth != depth {
        return None;
    }

    let root_choice = ((cert.payload >> (depth + 3)) & 1) as u8;
    let path_mask = if depth == 0 { 0 } else { (1u16 << depth) - 1 };
    let path = cert.payload & path_mask;
    let root = if root_choice == 0 { 0b101u16 } else { 0b110u16 };
    Some(Word {
        bits: (root << depth) | path,
        width: 3 + depth,
    })
}

fn verify(cert: Cert, expected: Word) -> bool {
    decode(cert) == Some(expected)
}

fn corpus() -> Vec<(Cert, Word, u8, u16)> {
    let mut out = Vec::new();
    for depth in 0u8..=5 {
        for root_choice in 0u8..=1 {
            for path in 0u16..(1u16 << depth) {
                let cert = encode(root_choice, depth, path).unwrap();
                let word = decode(cert).unwrap();
                out.push((cert, word, root_choice, path));
            }
        }
    }
    out
}

fn malformed_selftest() -> usize {
    let good = encode(0, 2, 0b01).unwrap();
    let expected = decode(good).unwrap();
    let mut rejected = 0usize;

    for bad in [
        Cert { payload: 0, bit_len: 3 },
        Cert { payload: 0, bit_len: 10 },
        Cert {
            payload: good.payload | (1u16 << good.bit_len),
            bit_len: good.bit_len,
        },
        Cert {
            payload: good.payload ^ (1u16 << 2),
            bit_len: good.bit_len,
        },
    ] {
        assert!(decode(bad).is_none());
        rejected += 1;
    }

    assert!(!verify(
        good,
        Word {
            bits: expected.bits ^ 1,
            width: expected.width,
        }
    ));
    rejected += 1;
    assert!(!verify(
        good,
        Word {
            bits: expected.bits,
            width: expected.width + 1,
        }
    ));
    rejected += 1;

    rejected
}

fn storage_stats() {
    println!("width\tcount\tflat_identity_bits_each\tself_framed_cert_bits_each\texternally_framed_cert_payload_bits_each\tflat_identity_bits_total\tself_framed_cert_bits_total\texternally_framed_cert_payload_bits_total");
    for width in 3u8..=8 {
        let depth = width - 3;
        let count = 1usize << (width - 2);
        let flat_each = width as usize;
        let self_framed_each = (4 + depth) as usize;
        let externally_framed_each = (1 + depth) as usize;
        println!(
            "{width}\t{count}\t{flat_each}\t{self_framed_each}\t{externally_framed_each}\t{}\t{}\t{}",
            count * flat_each,
            count * self_framed_each,
            count * externally_framed_each,
        );
    }
}

fn dump(corpus: &[(Cert, Word, u8, u16)]) {
    for &(cert, word, root_choice, path) in corpus {
        let depth = cert.bit_len - 4;
        println!(
            "CERT\troot_choice={root_choice}\tdepth={depth}\tpath={path}\tpayload={}\tbit_len={}\tresult_bits={}\tresult_width={}",
            cert.payload,
            cert.bit_len,
            word.bits,
            word.width,
        );
    }
}

fn bench_base(corpus: &[(Cert, Word, u8, u16)], reps: usize) -> u64 {
    let mut sum = 0u64;
    for _ in 0..reps {
        for i in 0..corpus.len() {
            sum = sum.wrapping_add(black_box(i as u64 & 1));
        }
    }
    black_box(sum)
}

fn bench_flat(corpus: &[(Cert, Word, u8, u16)], reps: usize) -> u64 {
    let mut sum = 0u64;
    for _ in 0..reps {
        for &(_, word, _, _) in corpus {
            let word = black_box(word);
            sum = sum
                .wrapping_mul(33)
                .wrapping_add(word.bits as u64)
                .wrapping_add(word.width as u64);
        }
    }
    black_box(sum)
}

fn bench_replay(corpus: &[(Cert, Word, u8, u16)], reps: usize) -> u64 {
    let mut sum = 0u64;
    for _ in 0..reps {
        for &(cert, _, _, _) in corpus {
            let word = decode(black_box(cert)).expect("valid selector certificate");
            sum = sum
                .wrapping_mul(33)
                .wrapping_add(word.bits as u64)
                .wrapping_add(word.width as u64);
        }
    }
    black_box(sum)
}

fn bench_verify(corpus: &[(Cert, Word, u8, u16)], reps: usize) -> u64 {
    let mut sum = 0u64;
    for _ in 0..reps {
        for &(cert, expected, _, _) in corpus {
            let ok = verify(black_box(cert), black_box(expected));
            assert!(ok);
            sum = sum.wrapping_add(ok as u64);
        }
    }
    black_box(sum)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("selftest");
    let reps = args
        .get(2)
        .and_then(|x| x.parse::<usize>().ok())
        .unwrap_or(1);

    let corpus = corpus();
    assert_eq!(corpus.len(), 126);
    assert_eq!(corpus.iter().filter(|(_, word, _, _)| word.width == 8).count(), 64);
    for &(cert, expected, _, _) in &corpus {
        assert!(verify(cert, expected));
    }

    match mode {
        "selftest" => {
            let rejected = malformed_selftest();
            println!(
                "CERT_BENCH\tmode=selftest\tcount={}\treps=1\trejected={rejected}\tchecksum=0",
                corpus.len()
            );
        }
        "stats" => storage_stats(),
        "dump" => dump(&corpus),
        "base" | "flat" | "replay" | "verify" => {
            if reps == 0 {
                eprintln!("reps must be positive");
                return ExitCode::from(2);
            }
            let checksum = match mode {
                "base" => bench_base(&corpus, reps),
                "flat" => bench_flat(&corpus, reps),
                "replay" => bench_replay(&corpus, reps),
                "verify" => bench_verify(&corpus, reps),
                _ => unreachable!(),
            };
            println!(
                "CERT_BENCH\tmode={mode}\tcount={}\treps={reps}\tchecksum={checksum}",
                corpus.len()
            );
        }
        _ => {
            eprintln!("usage: selector_certificate_bench [selftest|stats|dump|base|flat|replay|verify] [reps]");
            return ExitCode::from(2);
        }
    }

    ExitCode::SUCCESS
}
