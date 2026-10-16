//! Matched Frame3 Carrier benchmark for Zig f3c_bench.zig.
//! Physical encode/decode only; no claim to define SENS semantics.
use std::hint::black_box;
use std::time::Instant;
use sens::frame3_small::{encode_explicit_small_f3, decode_explicit_small_f3};

fn hex(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        s.push(ALPHABET[(byte >> 4) as usize] as char);
        s.push(ALPHABET[(byte & 15) as usize] as char);
    }
    s
}

fn trial(name: &str, op: &str, t5: &[u8], f3c: &[u8], reps: usize) {
    let start = Instant::now();
    for _ in 0..reps {
        match op {
            "encode" => {
                black_box(encode_explicit_small_f3(black_box(t5))
                    .expect("Rust F3C encode")
                    .expect("Rust F3C must save physical bytes"));
            }
            "decode" => {
                black_box(decode_explicit_small_f3(black_box(f3c))
                    .expect("Rust F3C decode"));
            }
            _ => panic!("unsupported operation"),
        }
    }
    println!("sample impl=rust case={name} op={op} elapsed_ns={} reps={reps}",
        start.elapsed().as_nanos());
}
fn main() {
    let cases: [(&str, &[u8], &[u8], usize); 2] = [
        ("quote", &[0x63, 0x89, 0x06, 0xa1],
         &[0xf3, 0x40, 0x60], 24),
        ("car_cons", &[0x66, 0x38, 0x67, 0x89, 0x89, 0x3b, 0x35],
         &[0xf3, 0x36, 0x5f, 0x1b, 0xd0, 0xc9], 12),
    ];
    for (name, input, expected, reps) in cases {
        let encoded = encode_explicit_small_f3(input)
            .expect("Rust encode")
            .expect("Rust must select tiny F3C");
        assert_eq!(encoded, expected, "physical F3C byte parity");
        let recovered = decode_explicit_small_f3(expected).expect("Rust decode");
        assert_eq!(recovered, input, "physical T5 restored");
        println!("physical impl=rust case={name} hex={}", hex(&encoded));
        println!("fixture impl=rust case={name} t5_bytes={} f3c_bytes={}",
            input.len(), encoded.len());
        for _ in 0..3 {
            black_box(encode_explicit_small_f3(input).unwrap());
            black_box(decode_explicit_small_f3(expected).unwrap());
        }
        for _ in 0..5 {
            trial(name, "encode", input, expected, reps);
            trial(name, "decode", input, expected, reps);
        }
    }
}
