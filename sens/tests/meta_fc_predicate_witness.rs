//! CI transport only: both predicates and their expected answers live in binary SENS.
//! No Rust truth table or independent semantic authority.
use sens::{eval_parsed_expressions, load_core_library, parse_mixed_exact_domain, ErrorKind, EvalResult, LanguageError, Session};

const LAW: &str = include_str!("../../../tests/semantic/meta-fc-predicates-d1-law.lisp");
const MUTANT: &str = include_str!("../../../tests/semantic/meta-fc-predicates-d1-mutant.lisp");

fn exact(source: &str, session: &mut Session) -> Result<EvalResult, LanguageError> {
    let forms = parse_mixed_exact_domain(source)?;
    eval_parsed_expressions(&forms, session)
}

fn session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session)
        .expect("test_id=meta-fc-d1: admitted exact-domain core must load");
    exact(include_str!("../../../lib/meta-eval-first-class.lisp"), &mut session)
        .expect("test_id=meta-fc-d1: source must load through exact-domain reader");
    session
}

#[test]
fn first_class_predicates_obey_sens_d1_law_and_reject_one_bit_mutant() {
    let changes: Vec<_> = LAW.bytes().zip(MUTANT.bytes())
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .collect();
    assert_eq!(LAW.len(), MUTANT.len(), "test_id=meta-fc-d1: fixture sizes drifted");
    assert_eq!(changes.len(), 1, "test_id=meta-fc-d1: mutant must flip one physical source bit");
    assert_eq!(changes[0].1, (b'0', b'1'), "test_id=meta-fc-d1: mutation must be D1:NO to D1:YES");

    let law = exact(LAW, &mut session())
        .unwrap_or_else(|err| panic!("test_id=meta-fc-d1-law layer=eval fixture=tests/semantic/meta-fc-predicates-d1-law.lisp reproduce=cargo test --locked -p sens --test meta_fc_predicate_witness -- --nocapture actual={err}"));
    assert_eq!(law.value.as_predicate_bit(), Some(true),
        "test_id=meta-fc-d1-law: only SENS D1:YES is a positive verdict");

    let error = exact(MUTANT, &mut session())
        .expect_err("test_id=meta-fc-d1-mutant: one-bit violation must invoke non-callable D1:NO");
    assert!(matches!(error.kind, ErrorKind::Type | ErrorKind::InvalidForm),
        "test_id=meta-fc-d1-mutant layer=eval expected=non-callable-D1 actual={error}");
}
