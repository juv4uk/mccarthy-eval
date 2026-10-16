//! Research witness for #3939.
//!
//! Proves that symbol-name conversion is a distinct value-level bridge from READ.
//! It does not ratify D8.

use sens::{eval_program, ErrorKind, Session, Value};
use std::rc::Rc;

fn eval(source: &str) -> Value {
    eval_program(source, &mut Session::default())
        .expect("symbol/text recovery witness should evaluate")
        .value
}

#[test]
fn symbol_and_string_name_bridges_are_inverse_on_current_carriers() {
    assert_eq!(
        eval(r#"(symbol->string (string->symbol "strange symbol"))"#),
        Value::String(Rc::from("strange symbol"))
    );
    assert_eq!(
        eval("(string->symbol (symbol->string (quote alpha)))"),
        Value::Symbol(Rc::from("alpha"))
    );
}

#[test]
fn string_to_symbol_is_not_derivable_as_plain_read_tokenization() {
    assert_eq!(
        eval(r#"(string->symbol "strange symbol")"#),
        Value::Symbol(Rc::from("strange symbol"))
    );

    let error = eval_program(r#"(read "strange symbol")"#, &mut Session::default())
        .expect_err("READ must reject two top-level forms");
    assert_eq!(error.kind, ErrorKind::InvalidForm);
}
