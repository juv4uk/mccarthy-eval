//! Contract 11.8 W1-W9 exact-width mechanical carrier benchmark.
//!
//! Measures source-word -> width-qualified DomainIdentity, not language execution.
//! W9 uses u16; the top bit must survive. No registry, semantic callable projection,
//! spelling, historical 8-bit authority or benchmark-local domain-law table.

use sens::{
    BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9, DomainIdentity,
};
use std::{env, hint::black_box, process::ExitCode};

fn source_word(width: u8, raw: u16) -> BinarySourceWord {
    let low = raw as u8;
    match width {
        1 => BinarySourceWord::W1(Bit1::new(low & 0b1).unwrap()),
        2 => BinarySourceWord::W2(Bit2::new(low & 0b11).unwrap()),
        3 => BinarySourceWord::W3(Bit3::new(low & 0b111).unwrap()),
        4 => BinarySourceWord::W4(Bit4::new(low & 0b1111).unwrap()),
        5 => BinarySourceWord::W5(Bit5::new(low & 0b1_1111).unwrap()),
        6 => BinarySourceWord::W6(Bit6::new(low & 0b11_1111).unwrap()),
        7 => BinarySourceWord::W7(Bit7::new(low & 0b111_1111).unwrap()),
        8 => BinarySourceWord::W8(Bit8::new(low).unwrap()),
        9 => BinarySourceWord::W9(Bit9::new(raw & 0x01ff).unwrap()),
        _ => unreachable!(),
    }
}

fn verify_invariants() {
    for width in 1..=9 {
        let source = source_word(width, 1);
        let identity = source.domain_identity();
        assert_eq!(identity.width(), width as usize);
        assert_eq!(identity.packed_bits(), 1);
        assert_eq!(identity.source_word(), source);
    }

    let same_payload = [
        source_word(1, 1).domain_identity(),
        source_word(2, 1).domain_identity(),
        source_word(3, 1).domain_identity(),
        source_word(4, 1).domain_identity(),
        source_word(5, 1).domain_identity(),
        source_word(6, 1).domain_identity(),
        source_word(7, 1).domain_identity(),
        source_word(8, 1).domain_identity(),
        source_word(9, 1).domain_identity(),
    ];
    for left in 0..same_payload.len() {
        for right in left + 1..same_payload.len() {
            assert_ne!(same_payload[left], same_payload[right]);
        }
    }

    // High W9 payload bit must not truncate to historical u8.
    let high = source_word(9, 0x101);
    let identity = DomainIdentity::from_source_word(high);
    assert_eq!(identity.width(), 9);
    assert_eq!(identity.packed_bits(), 0x101);
    assert_eq!(identity.source_word(), high);
    assert_ne!(identity, source_word(8, 1).domain_identity());
}

#[inline(never)]
fn run_empty(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        acc = acc.wrapping_add(black_box(i) & 1);
    }
    black_box(acc)
}

#[inline(never)]
fn run_domain(width: u8, iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let raw = black_box((i as u16) ^ ((i as u16) >> 1));
        let source = source_word(width, raw);
        let identity = DomainIdentity::from_source_word(source);
        acc = acc
            .wrapping_add(identity.packed_bits() as u64)
            .wrapping_add(identity.width() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_mixed(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let width = ((black_box(i) % 9) + 1) as u8;
        let source = source_word(width, (i as u16) ^ ((i as u16) >> 1));
        let identity = source.domain_identity();
        acc = acc
            .wrapping_add(identity.packed_bits() as u64)
            .wrapping_add(identity.width() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_width_roundtrip(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let width = ((black_box(i) % 9) + 1) as u8;
        let source = source_word(width, (i as u16) ^ ((i as u16) >> 1));
        let identity = DomainIdentity::from_source_word(source);
        let restored = identity.source_word();
        acc = acc
            .wrapping_add(restored.packed_bits() as u64)
            .wrapping_add(restored.width() as u64);
    }
    black_box(acc)
}

fn run_case(case: &str, iterations: u64) -> Option<u64> {
    Some(match case {
        "empty" => run_empty(iterations),
        "d1" => run_domain(1, iterations),
        "d2" => run_domain(2, iterations),
        "d3" => run_domain(3, iterations),
        "d4" => run_domain(4, iterations),
        "d5" => run_domain(5, iterations),
        "d6" => run_domain(6, iterations),
        "d7" => run_domain(7, iterations),
        "d8" => run_domain(8, iterations),
        "d9" => run_domain(9, iterations),
        "mixed" => run_mixed(iterations),
        "width-roundtrip" => run_width_roundtrip(iterations),
        _ => return None,
    })
}

fn main() -> ExitCode {
    verify_invariants();

    let mut args = env::args().skip(1);
    let case = args.next().unwrap_or_else(|| "empty".to_owned());
    let iterations = args
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(200_000);

    let Some(checksum) = run_case(&case, iterations) else {
        eprintln!("unknown case: {case}");
        return ExitCode::from(2);
    };

    println!("case={case} iterations={iterations} checksum={checksum}");
    ExitCode::SUCCESS
}
