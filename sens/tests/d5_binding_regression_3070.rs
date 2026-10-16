//! #3070 — bootstrap routing must converge on one Lisp-owned D5 mechanism.

use sens::{
    eval_parsed_expressions, eval_program, load_core_library, parse, Bit5, CoreD5, DomainIdentity,
    ErrorKind, Expr, ExprKind, Session, Span,
};

fn exact_d5_call(bits: u8, args: &str, session: &mut Session) -> String {
    let mut parsed = parse(&format!("(__d5_probe__ {args})")).expect("probe payload");
    let mut form = parsed.remove(0);
    let ExprKind::List(items) = form.kind else { panic!("probe list"); };
    let mut items = items.to_vec();
    items[0] = Expr {
        kind: ExprKind::DomainIdentity(DomainIdentity::D5(CoreD5::from_word(
            Bit5::new(bits).unwrap(),
        ))),
        span: Span::default(),
    };
    form.kind = ExprKind::List(items.into());
    eval_parsed_expressions(&[form], session)
        .expect("exact D5 call")
        .value
        .to_string()
}

#[test]
fn reverse_surface_legacy_code_and_exact_d5_share_one_bootstrapped_mechanism() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core bootstrap");

    assert_eq!(
        eval_program("(reverse '(a b c))", &mut session)
            .expect("surface reverse")
            .value
            .to_string(),
        "(c b a)"
    );

    assert_eq!(
        eval_program("(00101010 '(a b c))", &mut session)
            .expect("historical reverse code")
            .value
            .to_string(),
        "(c b a)"
    );

    assert_eq!(exact_d5_call(0b10100, "'(a b c)", &mut session), "(c b a)");
}

#[test]
fn legacy_quotient_never_borrows_the_exact_d5_direct_primitive() {
    let mut bare = Session::default();
    let error = eval_program("(00010100 6 3)", &mut bare)
        .expect_err("legacy QUOTIENT must require its Lisp bootstrap binding");
    assert_eq!(error.kind, ErrorKind::Type);

    let mut loaded = Session::default();
    load_core_library(&mut loaded).expect("core bootstrap");
    assert_eq!(
        eval_program("(00010100 6 3)", &mut loaded)
            .expect("legacy QUOTIENT should borrow only the bound Lisp closure")
            .value
            .to_string(),
        "2"
    );
    assert_eq!(exact_d5_call(0b10111, "6 3", &mut loaded), "2");
}
