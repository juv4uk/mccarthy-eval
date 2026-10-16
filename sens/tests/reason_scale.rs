//! Exercises lib/reason.my at scale (MYLISP-REASON-SCALE-PROFILE).
//!
//! Historical context: before 2026-09-07 `prove-goal` scanned rules through a
//! non-tail `append(... recursive-scan ...)` and overflowed the ordinary test
//! thread stack before/around N=100. That defect is repaired. This harness now
//! profiles the CURRENT tail-safe scan directly on the default stack.
//!
//! Wall-clock timings are printed for diagnosis only and are never asserted:
//! shared CI timing is noisy and is not a semantic contract. The stable claims
//! here are functional correctness and successful completion on the ordinary
//! stack. Use the ignored extended test manually when larger measurements are
//! needed before a further indexing/representation change.

use sens::{eval_program, Session};
use std::time::Instant;

fn loaded_session() -> Session {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/unify.lisp"), &mut session).unwrap();
    eval_program(include_str!("../../../lib/reason.lisp"), &mut session).unwrap();
    session
}

fn eval_session(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

/// Install a quoted linear edge chain without using Lisp recursion during
/// setup, so the timed measurement isolates `reason` instead of a recursive
/// fixture builder.
fn install_chain(session: &mut Session, n: usize) {
    let mut source = format!("(def chain{n} (quote (");
    for i in 0..n {
        source.push_str(&format!("((edge {i} {}))", i + 1));
    }
    source.push_str(")))");
    eval_session(session, &source);
}

/// Time a single query whose only match is the final fact, forcing a full
/// no-index scan. Chain construction is deliberately outside the timed region.
fn time_scan(session: &mut Session, n: usize) -> (u128, usize) {
    install_chain(session, n);
    let source =
        format!("(length (reason (list (quote edge) (logic-var (quote x)) {n}) chain{n}))");
    let start = Instant::now();
    let value = eval_session(session, &source).to_string();
    let elapsed_ns = start.elapsed().as_nanos();
    (elapsed_ns, value.trim().parse().unwrap_or(0))
}

fn profile_sizes(sizes: &[usize]) {
    let mut session = loaded_session();
    let mut table = String::from("reason scale profile (edge chain, full scan, default stack)\n");
    table.push_str("  N      elapsed_ns      results\n");

    for &n in sizes {
        let (ns, results) = time_scan(&mut session, n);
        table.push_str(&format!("  {n:<6} {ns:>14} {results}\n"));
        assert_eq!(results, 1, "full-scan goal at N={n} should match one edge");
    }

    println!("\n{table}");
}

#[test]
fn reason_scale_profile_default_stack_100_500_1000() {
    // Regression against the historical stack failure: no custom stack is
    // used. The 100/500/1000 series also preserves continuity with the 2026-08-29
    // measurements without turning noisy timing ratios into assertions.
    profile_sizes(&[100, 500, 1000]);
}

#[test]
#[ignore = "manual extended profile before further indexing/representation changes"]
fn reason_scale_profile_extended_5000_10000() {
    // Run explicitly with:
    // cargo test -p sens --test reason_scale reason_scale_profile_extended -- --ignored --nocapture
    profile_sizes(&[5_000, 10_000]);
}
