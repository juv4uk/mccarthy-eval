//! Surviving, non-redundant UK/SA surface tests.
//!
//! TEST-ARCHITECTURE-1 step 3 removed most of this file's per-operator
//! tests: each was a single hardcoded example of "does this UK/SA name
//! resolve to the same runtime value as its EN counterpart", a mutation
//! `uk_surface_equivalence.rs`'s registry-driven sweep already kills
//! generically for every stable EN/UK pair. What remains here either (a)
//! exercises syntax forms the identity-based sweep can't check (quote/cond
//! aren't first-class values), (b) exercises names/mechanisms the sweep
//! does not cover (aliases absent from the registry, or SA identity, which
//! has no EN-comparison sweep), or (c) is the acceptance-program runner
//! pattern the owner explicitly wants kept.
//!
//! Removed and why (surviving test that kills the same mutation):
//! - uk_atom_predicates_correctly, uk_eq_compares_identity,
//!   uk_cons_car_cdr_roundtrip, uk_abs_works, uk_min_max_work,
//!   uk_mod_quotient_work, uk_not_works, uk_equal_works,
//!   uk_symbol_predicate_works, uk_string_predicate_works,
//!   uk_list_length_append_reverse, uk_ordinals_work, uk_map_works,
//!   uk_filter_works, uk_reduce_works, uk_string_operations,
//!   uk_and_en_produce_same_result -> all stable EN/UK pairs, killed by
//!   `uk_surface_equivalence.rs::every_stable_uk_surface_entry_resolves_to_its_declared_operation`.
//! - uk_subtraction_works, uk_multiplication_works, uk_division_works,
//!   3 of 5 assertions in uk_comparisons_work (менше?/більше?/рівне?)
//!   -> `runtime_peer_operators.rs::stable_operator_peers_exist_before_human_surface_libraries_load`
//!   (same IDs 1001/1002/1003/1014/1015/1016, behaviorally checked there).
//! - uk_addition_works, uk_and_sa_produce_same_result (додати/+/yoga, ID
//!   0104) -> `rivnopravnist_mov.rs::додавання_відділяє_людські_мови_від_спільного_двійкового_коду`.
//! - sa_arithmetic_works, sa_comparisons_work, sa_and_en_produce_same_result
//!   -> `runtime_peer_operators.rs` CASES already behaviorally check every
//!   one of those SA spellings (viyoga/guṇana/haraṇa/hīna?/adhika?/sama?)
//!   against their UK/symbolic peers.

use sens::{eval_program, load_core_library, Session};

fn load_surface_prerequisites(session: &mut Session) {
    for source in [
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/forward.lisp"),
        include_str!("../../../lib/knowledge.lisp"),
        include_str!("../../../lib/persistent-map.lisp"),
        include_str!("../../../lib/persistent-vector.lisp"),
        include_str!("../../../lib/time.lisp"),
        include_str!("../../../lib/epistemic.lisp"),
    ] {
        eval_program(source, session).expect("surface prerequisite should load");
    }
}

/// Load core library + Ukrainian surface, return a fresh session.
fn uk_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core bootstrap");
    load_surface_prerequisites(&mut session);
    eval_program(include_str!("../../../lib/surface/uk.lisp"), &mut session)
        .expect("Ukrainian surface should load");
    session
}

/// Load core library + Sanskrit surface, return a fresh session.
fn sa_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core bootstrap");
    load_surface_prerequisites(&mut session);
    eval_program(include_str!("../../../lib/surface/sa.lisp"), &mut session)
        .expect("Sanskrit surface should load");
    session
}

// ── Syntax forms: not first-class values, so the identity-based
//    registry sweep in uk_surface_equivalence.rs can't check them. ──

#[test]
fn uk_quote_returns_form_unevaluated() {
    let mut s = uk_session();
    let r = eval_program("(як-є (+ 1 2))", &mut s).expect("eval");
    assert_eq!(r.value.to_string(), "(+ 1 2)");
}

#[test]
fn uk_cond_branches_correctly() {
    let mut s = uk_session();
    let r = eval_program(
        r#"(за-умовою
             ((тотожне? 1 1) (як-є так))
             ((тотожне? 1 1) (як-є ні)))"#,
        &mut s,
    )
    .expect("eval");
    assert_eq!(r.value.to_string(), "так");
}

// ── Comparison aliases not covered elsewhere: не-більше?/не-менше? have
//    no spelled-out EN name in the registry (EN column is "missing"), so
//    they fall outside both the equivalence sweep and runtime_peer_operators'
//    CASES table. менше?/більше?/рівне? are covered by
//    runtime_peer_operators.rs and were dropped from this test. ──

#[test]
fn uk_le_ge_comparisons_work() {
    let mut s = uk_session();
    assert_eq!(
        eval_program("(не-більше? 2 3)", &mut s)
            .unwrap()
            .value
            .to_string(),
        "1"
    );
    assert_eq!(
        eval_program("(не-менше? 3 3)", &mut s)
            .unwrap()
            .value
            .to_string(),
        "1"
    );
}

// ── Aliases absent from semantic-registry.lisp (за-номером/містить? are
//    plain `(define ... )` aliases in uk.lisp, not registry-tracked rows),
//    so no registry-driven sweep sees them. ──

#[test]
fn uk_legacy_nth_member_aliases_work() {
    let mut s = uk_session();
    let r = eval_program("(за-номером 1 (список 10 20 30))", &mut s).expect("eval");
    assert_eq!(r.value.to_string(), "20");
    let r2 = eval_program("(містить? 2 (список 1 2 3))", &mut s).expect("eval");
    assert_eq!(r2.value.to_string(), "t");
}

// ── Sanskrit surface: no SA/EN registry-driven equivalence sweep exists
//    yet (uk_surface_equivalence.rs only compares EN/UK); these svarūpa
//    (quote)/ādi (car)/śeṣa (cdr) and list/higher-order checks are the
//    only coverage for SA identity outside the specific IDs
//    runtime_peer_operators.rs's CASES table already hardcodes. ──

#[test]
fn sa_canon_works() {
    let mut s = sa_session();
    let r = eval_program("(svarūpa (+ 1 2))", &mut s).expect("eval");
    assert_eq!(r.value.to_string(), "(+ 1 2)");
    let r2 = eval_program("(ādi (saṃyuj 10 20))", &mut s).expect("eval");
    assert_eq!(r2.value.to_string(), "10");
    let r3 = eval_program("(śeṣa (saṃyuj 10 20))", &mut s).expect("eval");
    assert_eq!(r3.value.to_string(), "20");
}

#[test]
fn sa_lists_higher_order_work() {
    let mut s = sa_session();
    let r = eval_program("(śreṇī 1 2 3)", &mut s).expect("eval");
    assert_eq!(r.value.to_string(), "(1 2 3)");
    let r2 = eval_program("(pramāṇa (śreṇī 1 2 3))", &mut s).expect("eval");
    assert_eq!(r2.value.to_string(), "3");
    let r3 =
        eval_program("(āvartana (lambda (x) (guṇana x 2)) (śreṇī 1 2 3))", &mut s).expect("eval");
    assert_eq!(r3.value.to_string(), "(2 4 6)");
    let r4 =
        eval_program("(kalpana (lambda (x) (hīna? x 3)) (śreṇī 1 2 3 4))", &mut s).expect("eval");
    assert_eq!(r4.value.to_string(), "(1 2)");
}

// ── Ukrainian acceptance program: semantic truth lives in the `.my`
//    fixture, Rust just runs it -- a good pattern, not a duplication
//    target. Explicitly kept per the owner's guidance. ──

#[test]
fn uk_acceptance_program_passes() {
    let mut s = uk_session();
    let r = eval_program(
        include_str!("../../../lib/surface/uk-acceptance.lisp"),
        &mut s,
    )
    .expect("Ukrainian acceptance program should evaluate");
    assert_eq!(
        r.value.to_string(),
        "успіх",
        "Ukrainian acceptance program must return 'успіх (success)"
    );
}

/// Generic runner for `lib/surface/peer-identity-acceptance.my`
/// (2026-09-11, added alongside the abs/min/max/min-list/max-list
/// migration to lib/core.my): this test knows nothing about which
/// specific peer names are being checked or what their values should
/// be -- that semantic truth lives entirely in the `.my` file itself,
/// per the standing principle "semantic truth tests live with the
/// language; Rust tests only mechanism." Mirrors
/// `uk_acceptance_program_passes` immediately above.
#[test]
fn peer_identity_acceptance_program_passes() {
    let mut s = uk_session();
    let r = eval_program(
        include_str!("../../../lib/surface/peer-identity-acceptance.lisp"),
        &mut s,
    )
    .expect("peer-identity acceptance program should evaluate");
    assert_eq!(
        r.value.to_string(),
        "успіх",
        "Peer-identity acceptance program must return 'успіх (success)"
    );
}
