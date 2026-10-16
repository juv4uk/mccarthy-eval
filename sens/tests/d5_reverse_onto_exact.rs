//! #3379 — exact D5:10101 REVERSE-ONTO mechanism witness.

use sens::{
    eval_parsed_expressions, eval_program, load_core_library, parse, Bit5, CoreD5,
    DomainIdentity, Expr, ExprKind, Session, Span,
};

fn exact_reverse_onto(arguments_source: &str) -> String {
    let mut session = Session::default();
    load_core_library(&mut session).expect("active Core should load");

    let wrapped = format!("(__d5_reverse_onto__ {arguments_source})");
    let mut parsed = parse(&wrapped).expect("payload parses");
    let mut form = parsed.remove(0);
    let ExprKind::List(items) = form.kind else {
        panic!("wrapper must parse as list");
    };
    let mut items = items.to_vec();
    items[0] = Expr {
        kind: ExprKind::DomainIdentity(DomainIdentity::D5(CoreD5::from_word(
            Bit5::new(0b10101).unwrap(),
        ))),
        span: Span::default(),
    };
    form.kind = ExprKind::List(items.into());

    eval_parsed_expressions(&[form], &mut session)
        .expect("D5:10101 must execute")
        .value
        .to_string()
}

#[test]
fn reverse_onto_executes_from_exact_d5_identity() {
    assert_eq!(
        exact_reverse_onto("'(a b) '(c d)"),
        "(b a c d)"
    );
}

#[test]
fn compatibility_alias_and_exact_d5_share_behavior() {
    let exact = exact_reverse_onto("'(a b c) '(z)");

    let mut session = Session::default();
    load_core_library(&mut session).expect("active Core should load");
    let compatibility = eval_program(
        "(reverse-onto (quote (a b c)) (quote (z)))",
        &mut session,
    )
    .expect("compatibility alias should remain usable")
    .value
    .to_string();

    assert_eq!(exact, compatibility);
    assert_eq!(exact, "(c b a z)");
}
