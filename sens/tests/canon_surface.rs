use sens::{eval_program, ErrorKind, Session};

#[test]
fn ukrainian_canon_surface_is_reserved_but_ordinary_binding_is_lexical() {
    let mut session = Session::default();
    let error = eval_program("(def перше (lambda (x) (як-є затінено)))", &mut session)
        .expect_err("Contract 6.0 must reject shadowing a Ukrainian Canon spelling");
    assert_eq!(error.kind, ErrorKind::InvalidForm);

    let ordinary = eval_program(
        "(def локальна-функція (lambda (x y) (як-є локально))) (локальна-функція 1 2)",
        &mut session,
    )
    .expect("ordinary non-registry bindings must remain lexical values");
    assert_eq!(ordinary.value.to_string(), "локально");
}
