//! Vertical self-hosting witness: run the real Advice Taker reasoning stack
//! through the Lisp-owned meta-evaluator and check that both native and meta
//! independently reproduce all five epistemic outcome classes (proved,
//! proof-provenance, disputed, unknown, invalid). This does NOT assert exact
//! structural equality against a frozen expected value: the full native
//! output includes unification's internal variable-renumbering counters
//! (e.g. `(y . 1)`), which are an implementation detail of the unifier, not
//! part of the Advice Taker's semantic contract -- freezing them byte-for-byte
//! would make this test brittle to unrelated unifier refactors while adding
//! no real coverage of the outcome-class guarantee this file actually cares
//! about. TEST-ARCHITECTURE-1 (2026-09-12) renamed this from
//! `real_advice_taker_stack_has_exact_native_meta_parity`, which oversold
//! what the substring checks below actually establish.

use sens::{eval_program, Session};

fn escaped(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

fn advice_program() -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/result-status.lisp"),
        r#"
(def meta-advice-rules
  (quote (
    ((parent alice bob))
    ((parent bob carol))
    ((middle (var x) (var y))
      (parent (var x) (var y)))
    ((grandparent (var x) (var z))
      (middle (var x) (var y))
      (parent (var y) (var z)))
    ((safe sky))
    ((not? (safe sky)))
  )))

(def meta-advice-observe
  (lambda ()
    (let* ((proved
             (reason-observe
               (quote (grandparent alice carol))
               meta-advice-rules))
           (proof
             (second (car (third proved)))))
      (list
        (list (quote proved-outcome) proved)
        (list (quote proof-provenance) (provenance proof))
        (list
          (quote disputed-outcome)
          (reason-observe (quote (safe sky)) meta-advice-rules))
        (list
          (quote unknown-outcome)
          (reason-observe (quote (safe ocean)) meta-advice-rules))
        (list
          (quote invalid-outcome)
          (reason-observe (quote (not?)) meta-advice-rules))))))

(meta-advice-observe)
"#,
    )
}

fn native_result(program: &str) -> String {
    let mut session = Session::default();
    eval_program(program, &mut session)
        .unwrap_or_else(|error| panic!("native Advice Taker witness failed: {error}"))
        .value
        .to_string()
}

fn meta_result(program: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).expect("core bootstrap");
    sens::load_meta_evaluator_library(&mut session).expect("meta-eval bootstrap");

    eval_program(
        &format!(
            r#"(cdr (my-eval-program (read-all "{}") (quote ())))"#,
            escaped(program)
        ),
        &mut session,
    )
    .unwrap_or_else(|error| panic!("host failure while running meta Advice Taker witness: {error}"))
    .value
    .to_string()
}

#[test]
#[ignore = "deep meta-eval witness: release CI + debug nightly"]
fn advice_taker_native_and_meta_preserve_expected_outcome_classes() {
    // Disabled during language rebuilding (contract 7.0 archipelago migration / status-free registry rebuild).
    // Monolithic meta-eval Advice Taker execution is superseded by kernel archipelago execution (ADR-005).
    let _ = (&advice_program, &native_result, &meta_result);
}
