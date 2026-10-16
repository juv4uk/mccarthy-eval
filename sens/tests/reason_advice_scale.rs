//! B5 performance probe for the actual Advice Taker read path.
//!
//! Unlike `reason_scale.rs`, which isolates one worst-case full scan, this
//! harness keeps the knowledge journal/module projection and canonical
//! `reason-in-observe` adapter visible. Fixture installation is outside the
//! timed region and deliberately bypasses `advise-all`: atomic batch admission
//! has its own conflict-check complexity and must not contaminate a reasoning
//! measurement. Timings are diagnostic evidence only; semantic assertions
//! remain deterministic. Both timing profiles are ignored by default so
//! `cargo test --workspace` remains a correctness gate rather than a benchmark.
//!
//! The profile also times the public indexed `reason` path and an exact
//! forced-linear `prove-goal` path over the SAME projected rules and goal. This
//! makes the B5 indexing decision measurable without changing semantics or
//! comparing unrelated fixtures. `reason_index.rs` remains the stronger proof
//! that the two paths preserve exact result/proof structure.
//!
//! A second diagnostic slice asks several DIFFERENT questions about one
//! unchanged module. It compares `reason-in-observe` with the public
//! `reason-observe` entry supplied an explicitly prepared immutable
//! `reason-index/1`. This is not a cache: the caller owns a finite snapshot and
//! chooses when to prepare a newer one.

use sens::{eval_program, Session};
use std::time::Instant;

fn loaded_session() -> Session {
    let mut session = Session::default();
    for library in [
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/forward.lisp"),
        include_str!("../../../lib/knowledge.lisp"),
        include_str!("../../../lib/result-status.lisp"),
    ] {
        eval_program(library, &mut session).unwrap();
    }
    session
}

fn eval_session(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

/// Install a mixed module directly through the ordinary module/journal shape.
/// Distractor predicates model a heterogeneous knowledge base. The target is
/// intentionally placed after them and requires a two-rule derivation:
///
///   planet(target) -> has(target, mass) -> valuable(target)
///
/// Without predicate/head indexing, each proof level still scans the whole
/// projected clause list.
fn install_mixed_module(session: &mut Session, distractors: usize) -> usize {
    let mut source = String::from("(defmodule bench (quote (");
    for i in 0..distractors {
        match i % 4 {
            0 => source.push_str(&format!("((planet f{i}))")),
            1 => source.push_str(&format!("((orbits f{i} s{i}))")),
            2 => source.push_str(&format!("((has-mass f{i}))")),
            _ => source.push_str(&format!("((located-in f{i} sector{i}))")),
        }
    }
    source.push_str("((planet target))");
    source.push_str("((has (var x) mass) (planet (var x)))");
    source.push_str("((valuable (var x)) (has (var x) mass))");
    source.push_str(")))");

    eval_session(session, &source);
    distractors + 3
}

fn median_ns<F>(mut f: F) -> u128
where
    F: FnMut() -> u128,
{
    let mut samples = [f(), f(), f()];
    samples.sort_unstable();
    samples[1]
}

fn timed_eval(session: &mut Session, source: &str, expected: &str) -> u128 {
    let start = Instant::now();
    let value = eval_session(session, source);
    let elapsed = start.elapsed().as_nanos();
    assert_eq!(value, expected, "profile query changed semantics");
    elapsed
}

fn profile_size(distractors: usize) -> (u128, u128, u128, u128, u128, usize) {
    let mut session = loaded_session();
    let clauses = install_mixed_module(&mut session, distractors);

    let projection = "(length (module-clauses-now (quote bench)))";
    let expected_len = clauses.to_string();
    assert_eq!(eval_session(&mut session, projection), expected_len);

    // Pre-project once so the direct reasoning comparison excludes journal replay.
    eval_session(
        &mut session,
        "(def bench-rules (module-clauses-now (quote bench)))",
    );

    let indexed_direct = "(length (reason (quote (valuable target)) bench-rules))";
    let linear_direct = "(length (prove-goal (quote (valuable target)) bench-rules (quote ()) (reason-index-linear bench-rules) 0))";
    let raw_reason = "(result-status (reason-observe (quote (valuable target)) bench-rules))";
    let end_to_end = "(result-status (reason-in-observe (quote bench) (quote (valuable target))))";

    // Warm every measured reasoning path before the three-sample median. The
    // direct pair intentionally has the same projected corpus, goal and result
    // cardinality; only index selection differs.
    assert_eq!(eval_session(&mut session, indexed_direct), "1");
    assert_eq!(eval_session(&mut session, linear_direct), "1");
    assert_eq!(eval_session(&mut session, raw_reason), "proved");
    assert_eq!(eval_session(&mut session, end_to_end), "proved");

    let projection_ns = median_ns(|| timed_eval(&mut session, projection, &expected_len));
    let indexed_direct_ns = median_ns(|| timed_eval(&mut session, indexed_direct, "1"));
    let linear_direct_ns = median_ns(|| timed_eval(&mut session, linear_direct, "1"));
    let raw_reason_ns = median_ns(|| timed_eval(&mut session, raw_reason, "proved"));
    let end_to_end_ns = median_ns(|| timed_eval(&mut session, end_to_end, "proved"));

    (
        projection_ns,
        indexed_direct_ns,
        linear_direct_ns,
        raw_reason_ns,
        end_to_end_ns,
        clauses,
    )
}

fn profile_sizes(sizes: &[usize]) {
    let mut table = String::from(
        "Advice Taker B5 profile (3-sample median, default stack)\n\
         distractors clauses projection_ns indexed_ns linear_ns linear/indexed raw_reason_ns end_to_end_ns\n",
    );

    for &n in sizes {
        let (
            projection_ns,
            indexed_direct_ns,
            linear_direct_ns,
            raw_reason_ns,
            end_to_end_ns,
            clauses,
        ) = profile_size(n);
        let ratio = if indexed_direct_ns == 0 {
            0.0
        } else {
            linear_direct_ns as f64 / indexed_direct_ns as f64
        };
        table.push_str(&format!(
            "{n:<11} {clauses:<7} {projection_ns:<13} {indexed_direct_ns:<10} {linear_direct_ns:<10} {ratio:<14.2} {raw_reason_ns:<13} {end_to_end_ns}\n"
        ));
    }

    println!("\n{table}");
}

const REPEATED_GOALS: [&str; 6] = [
    "(valuable target)",
    "(planet f0)",
    "(orbits f1 s1)",
    "(has-mass f2)",
    "(located-in f3 sector3)",
    "(valuable missing)",
];

fn public_observe_source(goal: &str) -> String {
    format!("(reason-in-observe (quote bench) (quote {goal}))")
}

fn prepared_observe_source(goal: &str) -> String {
    format!("(reason-observe (quote {goal}) bench-index)")
}

fn timed_query_batch(session: &mut Session, prepared: bool) -> u128 {
    let start = Instant::now();
    for goal in REPEATED_GOALS {
        let source = if prepared {
            prepared_observe_source(goal)
        } else {
            public_observe_source(goal)
        };
        let _ = eval_session(session, &source);
    }
    start.elapsed().as_nanos()
}

fn timed_repeated_eval(session: &mut Session, source: &str, expected: &str) -> u128 {
    let start = Instant::now();
    for _ in REPEATED_GOALS {
        assert_eq!(eval_session(session, source), expected);
    }
    start.elapsed().as_nanos()
}

fn profile_repeated_queries(distractors: usize) {
    let mut session = loaded_session();
    let clauses = install_mixed_module(&mut session, distractors);

    eval_session(
        &mut session,
        "(def bench-rules (module-clauses-now (quote bench)))",
    );
    assert_eq!(
        eval_session(&mut session, "(length bench-rules)"),
        clauses.to_string()
    );
    eval_session(
        &mut session,
        "(def bench-index (reason-make-index bench-rules))",
    );
    assert_eq!(
        eval_session(&mut session, "(reason-index-mode bench-index)"),
        "indexed"
    );

    // Exact public-API outcome/proof parity for every distinct query before
    // timing anything. The prepared side passes the immutable index snapshot
    // through the same `reason-observe` entry point ordinary callers use.
    for goal in REPEATED_GOALS {
        let parity = format!(
            "(equal? {} {})",
            public_observe_source(goal),
            prepared_observe_source(goal)
        );
        assert_eq!(
            eval_session(&mut session, &parity),
            "(1)",
            "prepared public path changed outcome/proof structure for {goal}"
        );
    }

    // Warm both batch paths. The prepared path intentionally reuses the same
    // immutable projection/index across distinct questions.
    let _ = timed_query_batch(&mut session, false);
    let _ = timed_query_batch(&mut session, true);

    let public_batch_ns = median_ns(|| timed_query_batch(&mut session, false));
    let prepared_batch_ns = median_ns(|| timed_query_batch(&mut session, true));
    let projection_batch_ns = median_ns(|| {
        timed_repeated_eval(
            &mut session,
            "(length (module-clauses-now (quote bench)))",
            &clauses.to_string(),
        )
    });
    let index_batch_ns = median_ns(|| {
        timed_repeated_eval(
            &mut session,
            "(reason-index-mode (reason-make-index bench-rules))",
            "indexed",
        )
    });

    let speedup = if prepared_batch_ns == 0 {
        0.0
    } else {
        public_batch_ns as f64 / prepared_batch_ns as f64
    };

    println!(
        "\nAdvice Taker B5 repeated-query profile ({} distinct goals, {clauses} clauses, 3-sample median)\n\
         public_batch_ns   prepared_batch_ns   public/prepared   repeated_projection_ns   repeated_index_ns\n\
         {public_batch_ns:<17} {prepared_batch_ns:<19} {speedup:<17.2} {projection_batch_ns:<24} {index_batch_ns}\n",
        REPEATED_GOALS.len()
    );
}

#[test]
#[ignore = "B5 diagnostic profile; run explicitly with --ignored --nocapture"]
fn advice_taker_profile_100_500_1000_distractors() {
    profile_sizes(&[100, 500, 1_000]);
    // One medium-size same-module batch is enough to decide whether repeated
    // reconstruction deserves a separate design experiment; larger reuse
    // profiles remain unnecessary until this measurement says otherwise.
    profile_repeated_queries(500);
}

#[test]
#[ignore = "manual B5 extended profile before further indexing/representation changes"]
fn advice_taker_profile_5000_10000_distractors() {
    // Run explicitly with:
    // cargo test -p sens --test reason_advice_scale advice_taker_profile_5000_10000 -- --ignored --nocapture
    profile_sizes(&[5_000, 10_000]);
}
