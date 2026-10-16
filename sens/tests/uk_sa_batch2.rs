//! Surviving, non-redundant UK/SA batch-2 surface tests.
//!
//! TEST-ARCHITECTURE-1 step 3 removed the tests below whose UK name is a
//! stable EN/UK registry pair, already killed generically by
//! `uk_surface_equivalence.rs`'s sweep:
//! - uk_persistent_map_basic (порожня-карта/map-empty, ключ-у-карті?/map-contains?,
//!   вставити-в-карту/map-insert, отримати-з-карти/map-get)
//! - uk_persistent_vector_basic (розмір-вектора/vec-count,
//!   додати-до-вектора/vec-conj, елемент-вектора-за-індексом/vec-nth)
//! - uk_time_utc_now_works (поточний-всч/utc-now)
//! - uk_time_mono_ms_works (монотонний-мс/mono-ms)
//! - uk_describe_works (описати/describe)
//! - uk_unify_basic (уніфікувати/unify)
//! - uk_claim_predicate (твердження?/claim?)
//! - uk_batch2_en_equivalence (map-get/map-insert/map-empty, same pairs as
//!   uk_persistent_map_basic above)
//!
//! All killed by
//! `uk_surface_equivalence.rs::every_stable_uk_surface_entry_resolves_to_its_declared_operation`,
//! which verified each of those names is in fact a stable EN/UK registry
//! pair before this file's coverage was removed.
//!
//! sa_io_eval_works was dropped as a strict subset of sa_batch2_en_equivalence
//! below (both prove `vicāraṇa` evaluates a quoted form; the survivor also
//! checks EN parity).
//!
//! What remains is SA-only coverage with no EN/SA registry-driven sweep to
//! subsume it (uk_surface_equivalence.rs only compares EN/UK).

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

/// Load core + time + Sanskrit surface, return a fresh session.
fn sa_session_full() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core bootstrap");
    load_surface_prerequisites(&mut session);
    eval_program(include_str!("../../../lib/surface/sa.lisp"), &mut session)
        .expect("Sanskrit surface should load");
    session
}

// ── Sanskrit Batch 2: I/O ──────────────────────────────────────

#[test]
fn sa_env_works() {
    let mut s = sa_session_full();
    let r = eval_program("(āśraya)", &mut s).expect("eval");
    // env returns a list — just check it doesn't error
    assert!(!r.value.to_string().is_empty());
}

// ── Sanskrit Batch 2: Conversions ──────────────────────────────

#[test]
fn sa_conversions_work() {
    let mut s = sa_session_full();
    let r = eval_program("(saṅkhyā-śabda 42)", &mut s).expect("eval");
    assert_eq!(r.value.to_string(), "\"42\"");
    let r2 = eval_program("(nāman-śabda (svarūpa кіт))", &mut s).expect("eval");
    assert_eq!(r2.value.to_string(), "\"кіт\"");
}

// ── Sanskrit Batch 2: Vectors ──────────────────────────────────

#[test]
fn sa_vector_works() {
    let mut s = sa_session_full();
    let r = eval_program("(samūha 1 2 3)", &mut s).expect("eval");
    // vector internal repr — just check no error
    assert!(!r.value.to_string().is_empty());
}

// ── Sanskrit Batch 2: Time ─────────────────────────────────────

#[test]
fn sa_time_unix_works() {
    let mut s = sa_session_full();
    let r = eval_program("(kāla-unix)", &mut s).expect("eval");
    assert!(r.value.to_string().contains("unix-time"));
}

// ── Cross-surface: SA Batch 2 = EN ─────────────────────────────

#[test]
fn sa_batch2_en_equivalence() {
    let mut s = sa_session_full();
    let sa_r = eval_program("(vicāraṇa (svarūpa (* 3 4)))", &mut s).expect("eval");
    let en_r = eval_program("(eval (quote (* 3 4)))", &mut s).expect("eval");
    assert_eq!(sa_r.value.to_string(), en_r.value.to_string());
    assert_eq!(sa_r.value.to_string(), "12");
}
