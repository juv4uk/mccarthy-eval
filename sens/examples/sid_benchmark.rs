//! Paired surface-versus-SID microbenchmark for Canon list primitives.
//!
//! Run with:
//! `cargo run --release -p sens --example sid_benchmark`
//!
//! The source case includes the `(binary 8)` declaration because it measures
//! a complete standalone SID source unit. The AST case excludes that metadata
//! form and therefore isolates evaluator dispatch after parsing.

use sens::{
    eval_parsed_expressions, eval_program, load_core_library, parse, Session,
};
use std::{hint::black_box, time::Instant};

const SURFACE_SOURCE: &str = "(car (cons 1 (cons 2 ())))";
const SID_SOURCE: &str = "(binary 8) (00000101 (00000100 1 (00000100 2 ())))";
const SAMPLES: usize = 5;
const ITERATIONS: usize = 20_000;

fn median_ns(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn measure(operation: &mut impl FnMut()) -> f64 {
    for _ in 0..1_000 {
        operation();
    }
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        operation();
    }
    started.elapsed().as_nanos() as f64 / ITERATIONS as f64
}

fn sampled(mut operation: impl FnMut()) -> f64 {
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        samples.push(measure(&mut operation));
    }
    median_ns(samples)
}

fn main() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library loads");

    let surface_result = eval_program(SURFACE_SOURCE, &mut session).expect("surface program");
    let sid_result = eval_program(SID_SOURCE, &mut session).expect("SID program");
    assert_eq!(surface_result.value, sid_result.value, "paired programs must agree");
    assert_eq!(surface_result.value.to_string(), "1");

    let surface_source_ns = sampled(|| {
        black_box(eval_program(SURFACE_SOURCE, &mut session).expect("surface source"));
    });
    let sid_source_ns = sampled(|| {
        black_box(eval_program(SID_SOURCE, &mut session).expect("SID source"));
    });

    let surface_forms = parse(SURFACE_SOURCE).expect("surface source parses");
    let sid_forms = parse(SID_SOURCE).expect("SID source parses");
    let sid_call = &sid_forms[1..];
    let surface_ast_ns = sampled(|| {
        black_box(
            eval_parsed_expressions(&surface_forms, &mut session).expect("surface AST evaluation"),
        );
    });
    let sid_ast_ns = sampled(|| {
        black_box(eval_parsed_expressions(sid_call, &mut session).expect("SID AST evaluation"));
    });

    println!("SID_BENCH\titerations\t{ITERATIONS}\tsamples\t{SAMPLES}");
    println!("SID_BENCH\tsource-surface\t{surface_source_ns:.2}\tns/op");
    println!("SID_BENCH\tsource-sid\t{sid_source_ns:.2}\tns/op");
    println!(
        "SID_BENCH\tsource-sid/surface\t{:.4}\tratio",
        sid_source_ns / surface_source_ns
    );
    println!("SID_BENCH\tast-surface\t{surface_ast_ns:.2}\tns/op");
    println!("SID_BENCH\tast-sid\t{sid_ast_ns:.2}\tns/op");
    println!(
        "SID_BENCH\tast-sid/surface\t{:.4}\tratio",
        sid_ast_ns / surface_ast_ns
    );
}
