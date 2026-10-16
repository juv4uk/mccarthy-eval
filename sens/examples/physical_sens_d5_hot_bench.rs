//! Hot physical D5 benchmark: decode, canonical D2, and real in-process execution.
//!
//! The same bytes are accepted by production SENS CLI. All semantic interpretation
//! stays in the SENS evaluator. These timings do not measure process startup,
//! and agreeing entrypoints are NOT an independent language oracle.

use sens::{
    decode_ternary_words, eval_lowered_expressions, eval_parsed_expressions, lower_program,
    parse_canonical_word_sequence, Session,
};
use std::{env, fs, hint::black_box, time::Instant};

fn measure(mut f: impl FnMut(), iterations: usize, samples: usize) -> Vec<u128> {
    for _ in 0..3 {
        f();
    }
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        out.push(start.elapsed().as_nanos() / iterations as u128);
    }
    out
}

fn visible_hex(data: &str) -> String {
    data.as_bytes().iter().map(|c| format!("{c:02x}")).collect()
}

fn main() {
    let argv = env::args().collect::<Vec<_>>();
    assert_eq!(
        argv.len(), 5,
        "usage: physical_sens_d5_hot_bench FILE.sens CASE ITERATIONS SAMPLES"
    );
    let case = &argv[2];
    assert!(
        matches!(case.as_str(), "d5-label-recursion" | "d5-label-copy" | "d5-label-map"),
        "only admitted D5 program corpus may be measured"
    );
    let iterations: usize = argv[3].parse().expect("iterations");
    let samples: usize = argv[4].parse().expect("samples");
    assert!((1..=100_000).contains(&iterations));
    assert!((3..=101).contains(&samples) && samples % 2 == 1);

    let physical = fs::read(&argv[1]).expect("physical T5 file");
    let words = decode_ternary_words(&physical).expect("ratified T5 transport");
    let ast = parse_canonical_word_sequence(&words).expect("canonical D2");
    assert!(!ast.is_empty(), "empty D5 corpus");
    let lowered = lower_program(&ast);

    // Check the two real evaluator entrypoints before admitting timings.
    let expected = eval_parsed_expressions(&ast, &mut Session::default())
        .expect("D5 semantic execution");
    let lowered_expected = eval_lowered_expressions(&lowered, &mut Session::default())
        .expect("lowered D5 execution");
    assert_eq!(expected, lowered_expected, "D5 AST/lowered observable mismatch");
    let value = expected.value.to_string();
    let value_hex = visible_hex(&value);
    println!(
        "HOT_D5_OBSERVABLE\tcase={case}\tvalue_hex={value_hex}\tphysical_bytes={}\tword_count={}",
        physical.len(), words.len()
    );

    for phase in ["t5_decode_parse", "eval_parsed", "eval_lowered"] {
        let mut session = Session::default();
        let observed = match phase {
            "t5_decode_parse" => measure(
                || {
                    let decoded = decode_ternary_words(black_box(&physical))
                        .expect("measured physical T5 decoder");
                    black_box(
                        parse_canonical_word_sequence(&decoded)
                            .expect("measured D2 parser"),
                    );
                },
                iterations,
                samples,
            ),
            "eval_parsed" => {
                let first = eval_parsed_expressions(&ast, &mut session)
                    .expect("pre-measure D5 AST execution");
                assert_eq!(first, expected, "D5 observable changed before timing");
                let observations = measure(
                    || {
                        let answer = eval_parsed_expressions(black_box(&ast), &mut session)
                            .expect("repeated D5 AST evaluation");
                        black_box(answer);
                    },
                    iterations,
                    samples,
                );
                let last = eval_parsed_expressions(&ast, &mut session)
                    .expect("post-measure D5 AST execution");
                assert_eq!(last, expected, "D5 observable drift after timing");
                observations
            }
            "eval_lowered" => {
                let first = eval_lowered_expressions(&lowered, &mut session)
                    .expect("pre-measure lowered D5 execution");
                assert_eq!(first, expected, "lowered D5 observable changed before timing");
                let observations = measure(
                    || {
                        let answer = eval_lowered_expressions(black_box(&lowered), &mut session)
                            .expect("repeated lowered D5 evaluation");
                        black_box(answer);
                    },
                    iterations,
                    samples,
                );
                let last = eval_lowered_expressions(&lowered, &mut session)
                    .expect("post-measure lowered D5 execution");
                assert_eq!(last, expected, "lowered D5 observable drift after timing");
                observations
            }
            _ => unreachable!(),
        };
        let mut sorted = observed.clone();
        sorted.sort_unstable();
        let median = sorted[samples / 2];
        let p95 = sorted[(samples * 95).div_ceil(100) - 1];
        for (index, nanos) in observed.iter().enumerate() {
            println!(
                "HOT_D5_SAMPLE\tcase={case}\tphase={phase}\trep={}\tns_op={nanos}",
                index + 1
            );
        }
        println!(
            "HOT_D5_BENCH\tcase={case}\tphase={phase}\titerations={iterations}\tsamples={samples}\tmedian_ns_op={median}\tp95_ns_op={p95}"
        );
    }
}
