//! `print`/`princ`/`write-to-string`, `read`/`eval`/`read-all`, and the one
//! place in the crate that touches real stdin (`read_stdin_line`, behind
//! `(read)` with no arguments).
//!
//! These operations are value-level mechanisms. Their callable identities are
//! ordinary root `Value::Builtin` bindings; the evaluator does not dispatch on
//! their names.

use super::core::quoted;
use crate::eval::{closures, evaluate, lower};
use crate::{Environment, ErrorKind, Expr, LanguageError, Span, Value};
use std::rc::Rc;

pub(crate) fn print_values(
    arguments: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    exact_values("print", arguments, 1, span)?;
    let value = arguments[0].clone();
    environment.print(value.to_string());
    Ok(value)
}

pub(crate) fn princ_values(
    arguments: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    exact_values("princ", arguments, 1, span)?;
    let value = arguments[0].clone();
    environment.print(value.to_princ_string());
    Ok(value)
}

fn contains_predicate_bit(value: &Value) -> bool {
    match value {
        Value::DomainIdentity(crate::DomainIdentity::D1(_)) => true,
        Value::Pair(head, tail) => contains_predicate_bit(head) || contains_predicate_bit(tail),
        Value::Vector(values) => values.borrow().iter().any(contains_predicate_bit),
        _ => false,
    }
}

pub(crate) fn write_to_string_values(
    arguments: &[Value],
    _environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    exact_values("write-to-string", arguments, 1, span)?;
    if contains_predicate_bit(&arguments[0]) {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "write-to-string cannot serialize contextual predicate results",
            span,
        ));
    }
    Ok(Value::String(Rc::from(
        arguments[0].to_canonical_wire_string(),
    )))
}

pub(crate) fn read_values(
    arguments: &[Value],
    _environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    if arguments.len() > 1 {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            "read expects zero or one arguments · read ochikuie nul abo odyn arhument · read erwartet null oder ein Argument",
            span,
        ));
    }
    let source = if let Some(argument) = arguments.first() {
        match argument {
            Value::String(text) => text.to_string(),
            _ => {
                return Err(LanguageError::new(
                    ErrorKind::Type,
                    "read expects a string · read ochikuie riadok · read erwartet eine Zeichenkette",
                    span,
                ))
            }
        }
    } else {
        read_stdin_line(span)?
    };
    let expressions = crate::parse(&source).map_err(|mut error| {
        error.span = span;
        error
    })?;
    match <[Expr; 1]>::try_from(expressions) {
        Ok([expression]) => quoted(&expression),
        Err(expressions) => Err(LanguageError::new(
            ErrorKind::InvalidForm,
            format!(
                "read expects exactly one expression, found {} · read ochikuie rivno odyn vyraz, znaideno {} · read erwartet genau einen Ausdruck, gefunden {}",
                expressions.len(), expressions.len(), expressions.len()
            ),
            span,
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_stdin_line(span: Span) -> Result<String, LanguageError> {
    use std::io::BufRead;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).map_err(|error| {
        LanguageError::new(
            ErrorKind::InvalidForm,
            format!("read: failed to read from stdin · read: ne vdalos prochytaty stdin · read: Lesen von stdin fehlgeschlagen: {error}"),
            span,
        )
    })?;
    Ok(line.trim_end_matches(['\n', '\r']).to_string())
}

#[cfg(target_arch = "wasm32")]
fn read_stdin_line(span: Span) -> Result<String, LanguageError> {
    Err(LanguageError::new(
        ErrorKind::InvalidForm,
        "read: interactive stdin is not available in this build · read: interaktyvnyi stdin nedostupnyi u tsii zbirtsi · read: interaktives stdin ist in diesem Build nicht verfügbar",
        span,
    ))
}

pub(crate) fn eval_values(
    arguments: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    exact_values("eval", arguments, 1, span)?;
    let datum = arguments[0].clone();
    if matches!(datum, Value::Closure(_) | Value::Macro(_)) {
        return Ok(datum);
    }
    let expression = closures::value_to_expr(datum, span)?;
    let lowered = lower::lower_program(std::slice::from_ref(&expression));
    evaluate(&lowered[0], environment)
}

pub(crate) fn read_all_values(
    arguments: &[Value],
    _environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    exact_values("read-all", arguments, 1, span)?;
    let Value::String(text) = &arguments[0] else {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "read-all expects a string · read-all ochikuie riadok · read-all erwartet eine Zeichenkette",
            span,
        ));
    };
    let expressions = crate::parse(text).map_err(|mut error| {
        error.span = span;
        error
    })?;
    let mut values = Vec::with_capacity(expressions.len());
    for expression in &expressions {
        values.push(quoted(expression)?);
    }
    Ok(Value::list(values))
}

fn exact_values(
    name: &'static str,
    arguments: &[Value],
    expected: usize,
    span: Span,
) -> Result<(), LanguageError> {
    if arguments.len() != expected {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            format!("{name} expects exactly {expected} argument(s)"),
            span,
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_predicate_wire_rejected(value: Value) {
        let error = write_to_string_values(&[value], &Environment::root(), Span::default())
            .expect_err("PredicateBit must not enter canonical wire");
        assert_eq!(error.kind, ErrorKind::Type);
    }

    #[test]
    fn canonical_wire_rejects_predicate_bit_without_numeric_coercion() {
        assert_predicate_wire_rejected(Value::predicate_bit(true));
        assert_predicate_wire_rejected(Value::predicate_bit(false));
        assert_predicate_wire_rejected(Value::list([Value::predicate_bit(true)]));

        let vector = Value::Vector(Rc::new(std::cell::RefCell::new(vec![
            Value::Number(7.0, crate::Exactness::Exact),
            Value::predicate_bit(false),
        ])));
        assert_predicate_wire_rejected(vector);

        let number = write_to_string_values(
            &[Value::Number(1.0, crate::Exactness::Exact)],
            &Environment::root(),
            Span::default(),
        )
        .expect("ordinary exact Number wire remains supported");
        assert_eq!(number, Value::String(Rc::from("#q2:1/1")));
    }
}

