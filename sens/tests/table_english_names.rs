//! Власник, 2026-09-26: англійські назви в таблиці функцій для математики
//! (Lisp 1.5), `->`/`->>` і нове ділення з остачею `divmod` (Lisp 1.5 DIVIDE).
//! Кожна назва — той самий код, що й символ.

use sens::{eval_program, load_core_library, Session};

fn eval(source: &str) -> String {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    eval_program(source, &mut session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn english_math_names_are_the_same_code_as_the_symbols() {
    for (word, symbol, args) in [
        ("plus", "+", "7 5"),
        ("difference", "-", "7 5"),
        ("times", "*", "7 5"),
        ("divide", "/", "7 5"),
        ("lessp?", "<", "7 5"),
        ("greaterp?", ">", "7 5"),
        ("equalp?", "=", "7 5"),
        ("not-greaterp?", "<=", "7 5"),
        ("not-lessp?", ">=", "7 5"),
    ] {
        assert_eq!(
            eval(&format!("({word} {args})")),
            eval(&format!("({symbol} {args})")),
            "{word} must mean exactly {symbol}"
        );
    }
}

#[test]
fn divmod_returns_quotient_and_remainder() {
    assert_eq!(eval("(divmod 17 5)"), "(3 2)");
    assert_eq!(eval("(поділити-з-остачею 17 5)"), "(3 2)");
    assert_eq!(eval("(divmod 15 5)"), "(3 0)");
}

#[test]
fn thread_macros_have_english_names() {
    assert_eq!(
        eval("(thread-first 5 (- 2))"),
        eval("(-> 5 (- 2))")
    );
    assert_eq!(
        eval("(thread-last 5 (- 2))"),
        eval("(->> 5 (- 2))")
    );
}
