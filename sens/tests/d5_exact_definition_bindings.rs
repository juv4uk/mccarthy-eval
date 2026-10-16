//! #3375 — existing Lisp definitions supply mechanisms to ratified exact D5 identities.

use sens::{
    eval_parsed_expressions, load_core_library, parse, Bit5, CoreD5, DomainIdentity, Expr,
    ExprKind, Session, Span,
};

fn run_exact(bits: u8, args_source: &str) -> Result<String, String> {
    let mut session = Session::default();
    load_core_library(&mut session).map_err(|e| format!("load-core: {e:?}"))?;

    let wrapped = format!("(__d5__ {args_source})");
    let mut parsed = parse(&wrapped).map_err(|e| format!("parse: {e:?}"))?;
    let mut form = parsed.remove(0);
    let ExprKind::List(items) = form.kind else {
        return Err("wrapper-not-list".into());
    };
    let mut items = items.to_vec();
    items[0] = Expr {
        kind: ExprKind::DomainIdentity(DomainIdentity::D5(
            CoreD5::from_word(Bit5::new(bits).unwrap()),
        )),
        span: Span { start: 0, end: "__d5__".len() },
    };
    form.kind = ExprKind::List(items.into());

    eval_parsed_expressions(&[form], &mut session)
        .map(|o| o.value.to_string())
        .map_err(|e| format!("{:?}: {}", e.kind, e.message))
}

#[test]
fn exact_d5_existing_definitions_execute() {
    assert_eq!(run_exact(0b10100, "'(a b c)").unwrap(), "(c b a)");
    assert_eq!(run_exact(0b10101, "'(a b) '(c d)").unwrap(), "(b a c d)");
    assert_eq!(run_exact(0b10111, "6 3").unwrap(), "2");
    assert_eq!(run_exact(0b11100, "'b '((a . 1) (b . 2))").unwrap(), "(b . 2)");
    assert_eq!(run_exact(0b11101, "'b '(a b c)").unwrap(), "1");
    assert_eq!(run_exact(0b11111, "'x 'b '(a b c)").unwrap(), "(a x c)");
}
