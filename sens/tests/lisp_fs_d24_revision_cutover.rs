//! Інтеграційна перевірка: ревізія FS збільшується тільки через D6 ADD1
//! над явно кваліфікованими значеннями D24+.
//! Початковий fs-empty все ще містить історичний 0; цей тест не приписує
//! йому неіснуючого source-конструктора D24 і не послаблює арифметичний закон.

use sens::{BinaryNumber, ErrorKind, Session, Value, eval_program};
use std::rc::Rc;

fn load_filesystem() -> Session {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source(
        "lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session,
    ).expect("ядро");
    for (path, source) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp")),
        ("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp")),
        ("lib/world.lisp", include_str!("../../../lib/world.lisp")),
        ("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp")),
        ("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp")),
    ] {
        sens::eval_pinned_reasoning_source(path, source, &mut session)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
    }
    session
}

fn dnumber(width: usize, one: bool) -> Value {
    let mut bits = vec![0; width];
    if one {
        bits[width - 1] = 1;
    }
    Value::BinaryNumber(
        BinaryNumber::from_numeric_domain_bits(width, &bits)
            .expect("ратифікований числовий домен"),
    )
}

fn list(items: Vec<Value>) -> Value {
    items.into_iter().rev().fold(Value::Nil, |tail, head| {
        Value::Pair(Rc::new(head), Rc::new(tail))
    })
}

fn revision_of(snapshot: Value) -> Value {
    let Value::Pair(_, rest) = &snapshot else {
        panic!("snapshot: missing objects");
    };
    let Value::Pair(_, rest) = rest.as_ref() else {
        panic!("snapshot: missing bindings");
    };
    let Value::Pair(revision, tail) = rest.as_ref() else {
        panic!("snapshot: missing revision");
    };
    assert!(matches!(tail.as_ref(), Value::Nil), "snapshot has extra fields");
    revision.as_ref().clone()
}

#[test]
fn all_three_fs_increment_sites_have_exact_d6_heads() {
    let source = include_str!("../../../lib/lisp-fs.lisp");
    assert_eq!(source.matches("(001110 (fs-revision fs))").count(), 3);
    assert!(!source.contains("(01010 1 (fs-revision fs))"));
}

#[test]
fn successor_opcode_is_six_bits_and_never_conflates_with_five_bit_plus() {
    use sens::{parse_mixed_exact_domain, ExprKind};

    for (source, width, bits) in [
        ("(001110 revision)", 6usize, 0b001110u16),
        ("(01010 value revision)", 5usize, 0b01010u16),
    ] {
        let expressions = parse_mixed_exact_domain(source)
            .expect("the mixed migration reader must preserve source bit width");
        assert_eq!(expressions.len(), 1);
        let ExprKind::List(items) = &expressions[0].kind else {
            panic!("an operator application must remain a list");
        };
        let ExprKind::DomainIdentity(identity) = &items[0].kind else {
            panic!("an exact binary head may not become a legacy numeric ID");
        };
        assert_eq!(identity.width(), width, "{source}");
        assert_eq!(identity.packed_bits(), bits, "{source}");
    }
}

#[test]
fn fs_unbind_preserves_qualified_d24_and_d48_revisions() {
    for width in [24usize, 48] {
        let mut session = load_filesystem();
        session.environment.define(
            "snapshot",
            list(vec![Value::Nil, Value::Nil, dnumber(width, false)]),
        );
        let outcome = eval_program("(fs-unbind snapshot \"absent\")", &mut session)
            .expect("D6 ADD1 повинен приймати ревізію D24+");
        assert_eq!(
            revision_of(outcome.value),
            dnumber(width, true),
            "D{width}: ревізія має залишитися в тому самому числовому домені",
        );
    }
}

#[test]
fn fs_unbind_rejects_unqualified_revision() {
    let mut session = load_filesystem();
    session.environment.define(
        "snapshot",
        list(vec![Value::Nil, Value::Nil, Value::BinaryNumber(BinaryNumber::one())]),
    );
    let error = eval_program("(fs-unbind snapshot \"absent\")", &mut session)
        .expect_err("невідомий домен числа не можна припускати");
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(error.message.contains("D24+"), "{error}");
}
