//! #3379 — exact D5:11110 PAIRLIS mechanism witness.

use sens::{
    eval_parsed_expressions, eval_program, load_core_library, parse, Bit4, Bit5, CoreD4, CoreD5,
    DomainIdentity, Expr, ExprKind, Session, Span,
};

fn try_run_exact(identity: DomainIdentity, arguments_source: &str) -> Result<String, String> {
    let mut session = Session::default();
    load_core_library(&mut session)
        .map_err(|error| format!("{:?}: {}", error.kind, error.message))?;

    let wrapped = format!("(__domain_witness__ {arguments_source})");
    let mut parsed = parse(&wrapped).map_err(|error| format!("parse: {error:?}"))?;
    let mut form = parsed.remove(0);
    let ExprKind::List(items) = form.kind else {
        return Err("wrapper must parse as list".into());
    };
    let mut items = items.to_vec();
    items[0] = Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: Span::default(),
    };
    form.kind = ExprKind::List(items.into());

    eval_parsed_expressions(&[form], &mut session)
        .map(|outcome| outcome.value.to_string())
        .map_err(|error| format!("{:?}: {}", error.kind, error.message))
}

fn run_exact(identity: DomainIdentity, arguments_source: &str) -> String {
    try_run_exact(identity, arguments_source).expect("exact domain call must execute")
}

fn exact_pairlis(arguments_source: &str) -> String {
    run_exact(
        DomainIdentity::D5(CoreD5::from_word(Bit5::new(0b11110).unwrap())),
        arguments_source,
    )
}

#[test]
fn pairlis_executes_from_exact_d5_identity() {
    assert_eq!(
        exact_pairlis("'(x y) '(first second) '((z . third))"),
        "((x . first) (y . second) (z . third))"
    );
}

#[test]
fn canonical_surface_and_compatibility_alias_share_the_exact_d5_law() {
    let exact = exact_pairlis("'(x y) '(first second) '()");

    let mut session = Session::default();
    load_core_library(&mut session).expect("active Core should load");

    let canonical = eval_program(
        "(спарувати (quote (x y)) (quote (first second)) (quote ()))",
        &mut session,
    )
    .expect("canonical Ukrainian PAIRLIS surface should execute")
    .value
    .to_string();

    let compatibility = eval_program(
        "(pairlis (quote (x y)) (quote (first second)) (quote ()))",
        &mut session,
    )
    .expect("compatibility alias should remain usable")
    .value
    .to_string();

    assert_eq!(exact, "((x . first) (y . second))");
    assert_eq!(canonical, exact);
    assert_eq!(compatibility, exact);
}

#[test]
fn same_low_payload_in_d4_does_not_inherit_pairlis_meaning() {
    let args = "'(x y) '(first second) '((z . third))";
    let d5 = exact_pairlis(args);
    let d4_error = try_run_exact(
        DomainIdentity::D4(CoreD4::from_word(Bit4::new(0b1110).unwrap())),
        args,
    )
    .expect_err("D4:1110 has no admitted mechanism and must fail closed");

    assert!(
        d4_error.contains("no admitted") || d4_error.contains("not callable"),
        "unexpected D4 firewall error: {d4_error}"
    );
    assert_eq!(
        d5,
        "((x . first) (y . second) (z . third))",
        "D5:11110 remains the only PAIRLIS witness in this comparison"
    );
}
