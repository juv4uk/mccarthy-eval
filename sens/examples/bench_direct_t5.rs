//! Measurement-only pilot: separate physical decode, D2 admission,
//! visible-source baseline, and repeated interpretation on the SAME hardware.
//! Reports observations, never asserts a synthetic speedup or a native ISA.
use std::{hint::black_box, time::Instant};
use sens::{
    eval_parsed_expressions, lower_program, open_ternary_program, parse_canonical_binary,
    PhysicalT5Program, Session,
};
const QUOTE: &[u8] = include_bytes!("../../../tests/fixtures/migration-quote-cohort-main/quote-legacy.sens");
const ATOM: &[u8] = include_bytes!("../../../tests/fixtures/migration-atom-cohort/atom-empty.sens");
const COND: &[u8] = include_bytes!("../../../tests/fixtures/migration-d1-cond-cohort/branch.sens");

fn measure(mut task: impl FnMut(), count: usize) -> f64 {
    let start = Instant::now();
    for _ in 0..count {
        task();
    }
    start.elapsed().as_secs_f64() * 1e9 / count as f64
}

fn main() {
    let iterations: usize = std::env::args().nth(1)
        .map(|s| s.parse().expect("iterations must be a positive integer"))
        .unwrap_or(2_000);
    assert!((1..=1_000_000).contains(&iterations));
    eprintln!("measurement only; ns/op includes allocations and host overhead; iterations={iterations}");
    println!("fixture,bytes,forms,t5_decode_parse_lower_ns,visible_parse_lower_ns,cached_lowered_eval_ns");
    for (label, bytes) in [("quote", QUOTE), ("atom", ATOM), ("cond", COND)] {
        let program = PhysicalT5Program::decode(bytes).unwrap();
        let visible = open_ternary_program(bytes).unwrap();
        let reference = parse_canonical_binary(&visible).unwrap();
        let expected = eval_parsed_expressions(&reference, &mut Session::default()).unwrap();
        let actual = program.execute(&mut Session::default()).unwrap();
        assert_eq!(expected.value, actual.value);
        assert_eq!(expected.output, actual.output);
        // Untimed warmups. No reliance on timing to establish semantics.
        for _ in 0..100 {
            black_box(PhysicalT5Program::decode(black_box(bytes)).unwrap());
            black_box(lower_program(&parse_canonical_binary(black_box(&visible)).unwrap()));
            black_box(program.execute(&mut Session::default()).unwrap());
        }
        let direct_ns = measure(|| {
            black_box(PhysicalT5Program::decode(black_box(bytes)).unwrap());
        }, iterations);
        let visible_ns = measure(|| {
            black_box(lower_program(&parse_canonical_binary(black_box(&visible)).unwrap()));
        }, iterations);
        // Warm cached AST with a persistent session. In particular, do NOT
        // include construction/bootstrap of the runtime in this lane.
        let mut warmed_session = Session::default();
        let cached_ns = measure(|| {
            black_box(program.execute(black_box(&mut warmed_session)).unwrap());
        }, iterations);
        println!("{label},{},{},{direct_ns:.2},{visible_ns:.2},{cached_ns:.2}",
            bytes.len(), program.form_count());
    }
}
