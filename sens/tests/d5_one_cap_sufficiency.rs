use sens::{eval_program, ErrorKind, Session};

fn eval(source: &str) -> Result<String, sens::LanguageError> {
    let mut session = Session::default();
    eval_program(source, &mut session).map(|result| result.value.to_string())
}

#[test]
fn d5_one_cap_w1_transformer_preserves_unused_raw_operand() {
    let value = eval(
        r#"
        (define first-transformer
          (make-macro
            (lambda (a b) a)))
        (first-transformer (quote ok) never-defined)
        "#,
    )
    .expect("transformer surrogate must preserve unused raw operand");

    assert_eq!(value, "ok");
}

#[test]
fn d5_one_cap_w2_nested_transformer_expands_without_macroexpand_identity() {
    let value = eval(
        r#"
        (define inner-transformer
          (make-macro
            (lambda (x) x)))

        (define outer-transformer
          (make-macro
            (lambda (x)
              (cons
                (quote inner-transformer)
                (cons x (quote ()))))))

        (outer-transformer (quote ok))
        "#,
    )
    .expect("outer transformer should expand to an inner transformer call");

    assert_eq!(value, "ok");
}

#[test]
fn d5_one_cap_w3_expansion_runs_in_callers_lexical_environment() {
    let value = eval(
        r#"
        (define emit-local-x
          (make-macro
            (lambda () (quote local-x))))

        ((lambda (local-x)
           (emit-local-x))
         42)
        "#,
    )
    .expect("expanded symbol must resolve in caller lexical environment");

    assert_eq!(value, "42");
}

#[test]
fn d5_one_cap_w4_ordinary_lambda_remains_eager() {
    let error = eval(
        r#"
        (define ordinary-first
          (lambda (a b) a))
        (ordinary-first (quote ok) never-defined)
        "#,
    )
    .expect_err("ordinary lambda must evaluate the unused second operand eagerly");

    assert_eq!(error.kind, ErrorKind::UnknownSymbol);
    assert!(error.message.contains("never-defined"));
}

#[test]
fn d5_one_cap_w5_defmacro_remains_define_plus_transformer_derivation() {
    let macro_source = include_str!("../../../lib/macro.lisp");

    assert!(
        macro_source.contains("00001001"),
        "bootstrap macro derivation must still reference DEFINE"
    );
    assert!(
        macro_source.contains("00001000"),
        "bootstrap macro derivation must still reference LAMBDA"
    );
    assert!(
        macro_source.contains("make-macro"),
        "research surrogate must remain the single explicit transformer materializer"
    );

    let value = eval(
        r#"
        (defmacro derived-first (a b) a)
        (derived-first (quote ok) never-defined)
        "#,
    )
    .expect("language-owned defmacro should remain derived over the transformer surrogate");

    assert_eq!(value, "ok");
}
