use sens::{eval_program, Session};

fn loaded_session() -> Session {
    let mut session = Session::default();
    for library in [
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/forward.lisp"),
        include_str!("../../../lib/knowledge.lisp"),
    ] {
        eval_program(library, &mut session).expect("knowledge stack fixture should load");
    }
    session
}

fn eval(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn large_module_projection_and_existence_scan_are_stack_safe_on_default_thread() {
    const N: usize = 2_000;
    let mut session = loaded_session();

    let mut source = String::from("(defmodule bench (quote (");
    for i in 0..N {
        source.push_str(&format!("((item f{i}))"));
    }
    source.push_str(")))");
    eval(&mut session, &source);

    assert_eq!(eval(&mut session, "(module-known? (quote bench))"), "t");
    assert_eq!(
        eval(
            &mut session,
            "(length (module-journal-events (quote bench) *knowledge-journal*))",
        ),
        N.to_string()
    );
    assert_eq!(
        eval(&mut session, "(length (module-clauses-now (quote bench)))"),
        N.to_string()
    );

    // The tail-safe rewrite must preserve the historical projection order.
    assert_eq!(
        eval(
            &mut session,
            "(second (car (car (module-clauses-now (quote bench)))))",
        ),
        "f0"
    );
    assert_eq!(
        eval(
            &mut session,
            &format!(
                "(second (car (nth {} (module-clauses-now (quote bench)))))",
                N - 1
            ),
        ),
        format!("f{}", N - 1)
    );
}

