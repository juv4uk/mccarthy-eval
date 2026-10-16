//! Cachegrind micro-benchmark for the owner-ratified D1->D4 foundation.
//!
//! This executable measures the exact-width carrier path only. It does not
//! assign or redefine SENS semantics. D1-D4 use production exact-domain newtypes.

use sens::{Bija3, Bit1, Bit2, Bit3, Bit4, CoreD4, PredicateBit, Racana2};
use std::{env, hint::black_box, process::ExitCode};

#[derive(Clone, Copy)]
enum MixedWord {
    D1(PredicateBit),
    D2(Racana2),
    D3(Bija3),
    D4(CoreD4),
}

fn verify_foundation_mechanics() {
    for raw in 0..=1 {
        assert_eq!(PredicateBit::from_word(Bit1::new(raw).unwrap()).word().packed_bits(), raw);
    }
    for raw in 0..=3 {
        assert_eq!(Racana2::from_word(Bit2::new(raw).unwrap()).word().packed_bits(), raw);
    }
    for raw in 0..=7 {
        assert_eq!(Bija3::from_word(Bit3::new(raw).unwrap()).word().packed_bits(), raw);
    }
    for raw in 0..=15 {
        assert_eq!(CoreD4::from_word(Bit4::new(raw).unwrap()).word().packed_bits(), raw);
    }

    // Exact-width anti-collapse is carried by the Rust types, not numeric value.
    assert_eq!(Bit1::width(), 1);
    assert_eq!(Bit2::width(), 2);
    assert_eq!(Bit3::width(), 3);
    assert_eq!(Bit4::width(), 4);

    // D3 selector roots 101/110 generate the four ratified D4 selector words.
    let mut generated = Vec::new();
    for root_raw in [0b101, 0b110] {
        let root = Bit3::new(root_raw).unwrap();
        for suffix in [false, true] {
            let child = root.append::<4>(suffix).unwrap();
            assert!(child.parent::<3>() == Some(root));
            assert!(root.is_prefix_of(child));
            generated.push(child.packed_bits());
        }
    }
    assert_eq!(generated, vec![0b1010, 0b1011, 0b1100, 0b1101]);
}

#[inline(never)]
fn run_empty(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        acc = acc.wrapping_add(i & 1);
    }
    black_box(acc)
}

#[inline(never)]
fn run_d1(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let word = Bit1::new((i & 0b1) as u8).unwrap();
        let domain = PredicateBit::from_word(word);
        let word = domain.word();
        acc = acc
            .wrapping_add(word.packed_bits() as u64)
            .wrapping_add(word.bit(0).unwrap() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_d2(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let word = Bit2::new((i & 0b11) as u8).unwrap();
        let domain = Racana2::from_word(word);
        let word = domain.word();
        acc = acc
            .wrapping_add(word.packed_bits() as u64)
            .wrapping_add(word.bit(1).unwrap() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_d3(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let word = Bit3::new((i & 0b111) as u8).unwrap();
        let domain = Bija3::from_word(word);
        let word = domain.word();
        acc = acc
            .wrapping_add(word.packed_bits() as u64)
            .wrapping_add(word.bit(2).unwrap() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_d4(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let word = Bit4::new((i & 0b1111) as u8).unwrap();
        let domain = CoreD4::from_word(word);
        let word = domain.word();
        acc = acc
            .wrapping_add(word.packed_bits() as u64)
            .wrapping_add(word.bit(3).unwrap() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_ladder(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let d1 = Bit1::new((i & 1) as u8).unwrap();
        let d2 = d1.append::<2>((i & 2) != 0).unwrap();
        let d3 = d2.append::<3>((i & 4) != 0).unwrap();
        let d4 = d3.append::<4>((i & 8) != 0).unwrap();
        let parent = d4.parent::<3>().unwrap();
        acc = acc
            .wrapping_add(d4.packed_bits() as u64)
            .wrapping_add(parent.packed_bits() as u64)
            .wrapping_add(d3.is_prefix_of(d4) as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_selector(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let root = Bit3::new(if (i & 1) == 0 { 0b101 } else { 0b110 }).unwrap();
        let child = root.append::<4>((i & 2) != 0).unwrap();
        acc = acc
            .wrapping_add(child.packed_bits() as u64)
            .wrapping_add(child.bit(3).unwrap() as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_mixed(iterations: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..iterations {
        let i = black_box(i);
        let value = match i & 3 {
            0 => MixedWord::D1(PredicateBit::from_word(Bit1::new((i & 1) as u8).unwrap())),
            1 => MixedWord::D2(Racana2::from_word(Bit2::new((i & 3) as u8).unwrap())),
            2 => MixedWord::D3(Bija3::from_word(Bit3::new((i & 7) as u8).unwrap())),
            _ => MixedWord::D4(CoreD4::from_word(Bit4::new((i & 15) as u8).unwrap())),
        };
        let packed = match value {
            MixedWord::D1(x) => x.word().packed_bits(),
            MixedWord::D2(x) => x.word().packed_bits(),
            MixedWord::D3(x) => x.word().packed_bits(),
            MixedWord::D4(x) => x.word().packed_bits(),
        };
        acc = acc.wrapping_add(packed as u64);
    }
    black_box(acc)
}

#[inline(never)]
fn run_case(case: &str, iterations: u64) -> u64 {
    match case {
        "empty" => run_empty(iterations),
        "d1" => run_d1(iterations),
        "d2" => run_d2(iterations),
        "d3" => run_d3(iterations),
        "d4" => run_d4(iterations),
        "ladder" => run_ladder(iterations),
        "selector" => run_selector(iterations),
        "mixed" => run_mixed(iterations),
        _ => u64::MAX,
    }
}

fn main() -> ExitCode {
    verify_foundation_mechanics();

    let mut args = env::args().skip(1);
    let case = args.next().unwrap_or_else(|| "empty".to_owned());
    let iterations = args
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(200_000);

    if !matches!(
        case.as_str(),
        "empty" | "d1" | "d2" | "d3" | "d4" | "ladder" | "selector" | "mixed"
    ) {
        eprintln!("unknown case: {case}");
        return ExitCode::from(2);
    }

    let checksum = run_case(&case, iterations);
    println!("case={case} iterations={iterations} checksum={checksum}");
    ExitCode::SUCCESS
}
