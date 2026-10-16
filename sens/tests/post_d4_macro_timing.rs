use sens::{Session, eval_program};

fn eval(source: &str) -> String {
    let mut session = Session::default();
    eval_program(source, &mut session)
        .expect("timing witness must evaluate")
        .value
        .to_string()
}

#[test]
fn evaluation_time_transformer_observes_same_frame_redefinition_after_function_definition() {
    let value = eval(
        r#"
        (define phase-transformer
          (make-macro
            (lambda ()
              (quote (quote OLD)))))

        (define invoke-later
          (lambda ()
            (phase-transformer)))

        (define phase-transformer
          (make-macro
            (lambda ()
              (quote (quote NEW)))))

        (invoke-later)
        "#,
    );

    assert_eq!(
        value, "NEW",
        "current SENS expands the transformer call when invoke-later is evaluated,          so the same-frame transformer redefinition is visible"
    );
}

#[test]
fn evaluation_time_transformer_still_executes_generated_form_in_calling_environment() {
    let value = eval(
        r#"
        (define emit-caller-name
          (make-macro
            (lambda ()
              (quote caller-name))))

        ((lambda (caller-name)
           (emit-caller-name))
         73)
        "#,
    );

    assert_eq!(value, "73");
}
