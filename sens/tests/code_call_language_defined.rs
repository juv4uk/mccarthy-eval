//! #1455: код СЕНС (8 біт) викликає й функції таблиці, визначені мовою в
//! lib/core.lisp, а не лише примітиви Rust. Порядок: примітив → визначення
//! мовою, прив'язане до коду → помилка.

use sens::{eval_program, load_core_library, ErrorKind, Session};

fn core_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    session
}

fn eval(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn language_defined_functions_are_callable_by_code() {
    let mut session = core_session();
    // list = 00100111, member? = 00101100, quotient = 00010100, append = 00101001
    for (by_code, by_name) in [
        ("(00100111 1 2 3)", "(list 1 2 3)"),
        ("(00101100 2 (00000001 (1 2 3)))", "(member? 2 (00000001 (1 2 3)))"),
        ("(00010100 17 5)", "(quotient 17 5)"),
        ("(00101001 (00000001 (1 2)) (00000001 (3)))", "(append (00000001 (1 2)) (00000001 (3)))"),
    ] {
        assert_eq!(eval(&mut session, by_code), eval(&mut session, by_name), "{by_code}");
    }
}

#[test]
fn a_local_definition_does_not_bind_the_code() {
    let mut session = Session::default();
    // Без ядра: локальне визначення `list` усередині функції не стає механізмом коду.
    eval(
        &mut session,
        "(00001001 f (00001000 () (00001001 list (00001000 args 7)) 0)) (f)",
    );
    let error = eval_program("(00100111 1 2)", &mut session).expect_err("no mechanism yet");
    assert_eq!(error.kind, ErrorKind::Type);
}

#[test]
fn a_code_without_mechanism_fails_closed() {
    let mut session = core_session();
    // 11111110 — порожній рядок таблиці функцій (10110001 з #1391 уже зайнятий).
    let error = eval_program("(11111110 1)", &mut session).expect_err("empty code must fail");
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(error.message.contains("no admitted callable mechanism"));
}

#[test]
fn a_code_used_as_a_value_is_callable_through_its_definition() {
    let mut session = core_session();
    assert_eq!(
        eval(&mut session, "((00001000 (f) (f 1 2)) 00100111)"),
        "(1 2)"
    );
}

#[test]
fn a_macro_bound_to_a_code_expands_before_its_arguments_are_evaluated() {
    let mut session = core_session();
    // and = 10011010: другий аргумент не обчислюється, якщо перший хибний.
    assert_eq!(eval(&mut session, "(10011010 () (never-defined-function))"), "()");
    assert_eq!(
        eval(&mut session, "(10011010 1 2)"),
        eval(&mut session, "(and 1 2)")
    );
    // let = 10011100
    assert_eq!(eval(&mut session, "(10011100 ((x 40)) (00001100 x 2))"), "42");
}

#[test]
fn a_local_definition_of_a_table_name_does_not_retarget_the_code() {
    let mut session = core_session();
    // member? = 00101100; локальний define усередині функції не змінює код.
    assert_eq!(
        eval(
            &mut session,
            "(00001001 f (00001000 ()
               (00001001 member? (00001000 (x l) (00000001 hijacked)))
               (00101100 2 (00000001 (1 2 3)))))
             (f)"
        ),
        eval(&mut session, "(member? 2 (00000001 (1 2 3)))")
    );
}

#[test]
fn defmacro_itself_is_callable_by_its_code() {
    let mut session = core_session();
    // defmacro = 00001010: визначення макроса кодом, виклик — назвою й кодом.
    // (00001010 quoted (x) ...) розгортається в (quote x).
    eval(
        &mut session,
        "(00001010 quoted (x) (00000100 (00000001 00000001) (00000100 x (00000001 ()))))",
    );
    assert_eq!(eval(&mut session, "(quoted (a b))"), "(a b)");
}

