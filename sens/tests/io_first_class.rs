use sens::{eval_program, ErrorKind, Session, Value};
use std::rc::Rc;

fn eval(source: &str) -> Value {
    eval_program(source, &mut Session::default())
        .unwrap_or_else(|error| panic!("evaluation failed: {error}\nsource: {source}"))
        .value
}

#[test]
fn write_to_string_read_and_eval_are_first_class_values() {
    assert_eq!(
        eval("(def render 01001100) (render (quote (a b)))"),
        Value::String(Rc::from("(a b)")),
    );
    assert_eq!(
        eval("(def parse-one 01001010) (parse-one \"(+ 1 2)\")").to_string(),
        "(+ 1 2)",
    );
    assert_eq!(
        eval("(def run 01001101) (run (quote (00001100 1 2)))"),
        Value::Number(3.0, sens::Exactness::Exact),
    );
}

#[test]
fn read_all_is_a_first_class_value() {
    assert_eq!(
        eval("(def parse-all 01001011) (parse-all \"a b\")").to_string(),
        "(a b)",
    );
}

#[test]
fn print_and_princ_are_first_class_and_keep_transcript_semantics() {
    let mut print_session = Session::default();
    let print_result = eval_program("(def emit 01001000) (emit \"hello\")", &mut print_session)
        .expect("first-class print should evaluate");
    assert_eq!(print_result.value, Value::String(Rc::from("hello")));
    assert_eq!(print_result.output, vec!["\"hello\"".to_string()]);

    let mut princ_session = Session::default();
    let princ_result = eval_program("(def emit 01001001) (emit \"hello\")", &mut princ_session)
        .expect("first-class princ should evaluate");
    assert_eq!(princ_result.value, Value::String(Rc::from("hello")));
    assert_eq!(princ_result.output, vec!["hello".to_string()]);
}

#[test]
fn read_and_eval_preserve_named_failure_classes() {
    let read_type = eval_program("(read 42)", &mut Session::default())
        .expect_err("read must reject non-string input");
    assert_eq!(read_type.kind, ErrorKind::Type);

    let read_all_type = eval_program("(read-all 42)", &mut Session::default())
        .expect_err("read-all must reject non-string input");
    assert_eq!(read_all_type.kind, ErrorKind::Type);

    let eval_arity = eval_program("(eval)", &mut Session::default())
        .expect_err("eval must retain exact arity");
    assert_eq!(eval_arity.kind, ErrorKind::Arity);
}

#[test]
fn evaluator_source_does_not_dispatch_io_reflection_names() {
    let evaluator = include_str!("../src/eval/mod.rs");
    for name in [
        "print",
        "princ",
        "write-to-string",
        "read",
        "read-all",
        "eval",
    ] {
        assert!(
            !evaluator.contains(&format!("Some(\"{name}\")")),
            "evaluator regained hard-coded ownership of {name}"
        );
    }
}
