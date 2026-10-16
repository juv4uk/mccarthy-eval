//! The McCarthy primitives (`eq`, `car`, `cdr`, `cons`, `cond`, `quote`'s
//! helper), plus the compatibility `def` surface. Language-owned `defmacro`
//! is bootstrapped from `lib/macro.lisp`; the Rust kernel no longer implements it.

use crate::eval::canon;
use crate::eval::{evaluate, evaluate_step, EvalStep};
use crate::{Bit4, CoreD4, CoreDomainIdentity, Environment, ErrorKind, Expr, ExprKind, LanguageError, Span, Value};

use std::rc::Rc;

/// Mechanism-only ATOM result: D1 predicate, never a host Bool, list-scale,
/// Core profile, or historical T/NIL carrier.
/// The domain-word contracts own the meaning of the returned bit.
pub(crate) fn atom_value(value: &Value, _environment: &Environment) -> Value {
    Value::predicate_bit(!matches!(value, Value::Pair(_, _)))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExactD3CondControl {
    Select,
    SkipNo,
}

/// Canonical D3:110 control projection.
///
/// Contract 11.8: only exact D1 PredicateBit answers may control COND.
/// D1:1 selects and D1:0 skips. Structural (), Number 0/1, T/NIL and
/// every other non-D1 value are wrong-domain control inputs and fail closed.
fn exact_d3_cond_control(value: &Value, span: Span) -> Result<ExactD3CondControl, LanguageError> {
    match value.as_predicate_bit() {
        Some(true) => Ok(ExactD3CondControl::Select),
        Some(false) => Ok(ExactD3CondControl::SkipNo),
        None => Err(LanguageError::new(
            ErrorKind::Type,
            format!("D3:110 COND expects exact D1:1 / D1:0; got {value}"),
            span,
        )),
    }
}

/// Exact-domain D3:110 COND.
///
/// This path is deliberately separate from historical CoreDomainIdentity/profile COND.
/// It is two-part only and never consults host/Lisp truthiness.
pub(crate) fn evaluate_domain_cond(
    clauses: &[Expr],
    environment: &Environment,
    _span: Span,
) -> Result<EvalStep, LanguageError> {
    for clause in clauses {
        let ExprKind::List(parts) = &clause.kind else {
            return Err(LanguageError::new(
                ErrorKind::InvalidForm,
                "D3:110 COND expects list clauses",
                clause.span,
            ));
        };
        // Contract 11.8: canonical D3 COND has exactly two fields.
        // Reject old explicit-result clauses *before* evaluating their query:
        // evaluating a malformed clause would produce observable side effects.
        if parts.len() != 2 {
            return Err(LanguageError::new(
                ErrorKind::InvalidForm,
                "D3:110 COND requires exactly (test expression); three-part compatibility is forbidden",
                clause.span,
            ));
        }
        let value = evaluate(&parts[0], environment)?;
        match exact_d3_cond_control(&value, parts[0].span)? {
            ExactD3CondControl::Select => return evaluate_step(&parts[1], environment),
            ExactD3CondControl::SkipNo => {}
        }
    }

    Ok(EvalStep::Value(Value::Nil))
}

pub(crate) fn evaluate_definition(
    arguments: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    // DEFINE — точна D4:0011, а не історична W8:00001011.
    let identity = CoreDomainIdentity::D4(
        CoreD4::from_word(Bit4::new(0b0011).expect("ратифікована координата D4 DEFINE")),
    );
    exact_domain_arity(identity, arguments, 2, span)?;

    // A D2-framed W7 sequence is a Text7 identifier only in this explicit
    // DEFINE target role. The canonical reader remains structural and makes
    // no global W7→Text7 inference.
    if let Some(name) = crate::canonical_reader::text7_binding_key(&arguments[0]) {
        canon::ensure_bindable(&name, arguments[0].span)?;
        let value = evaluate(&arguments[1], environment)?;
        canon::bind_language_definition(&name, &value, environment);
        environment.define(name, value.clone());
        return Ok(value);
    }

    match &arguments[0].kind {
        ExprKind::Symbol(name) => {
            canon::ensure_bindable(name, arguments[0].span)?;
            let value = evaluate(&arguments[1], environment)?;
            // The shared lexical frame makes recursive definitions visible to their closure after binding.
            // Spilnyi leksychnyi freim robyt rekursyvne vyznachennia vydymym zamykanniu pislia zv’yazuvannia.
            // Der gemeinsame lexikalische Frame macht rekursive Definitionen nach der Bindung für ihre Closure sichtbar.
            canon::bind_language_definition(name, &value, environment);
            environment.define(name.clone(), value.clone());
            Ok(value)
        }
        ExprKind::DomainIdentity(identity) => {
            let Some(identity) = identity.core_operation() else {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    "exact DEFINE target must be a callable D3/D4/D5/D6 DomainIdentity",
                    arguments[0].span,
                ));
            };

            if !environment.is_root() {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    "exact DEFINE target may install a language-owned mechanism only at the root frame",
                    arguments[0].span,
                ));
            }

            if environment.domain_code_slot(identity).is_some() {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("exact DEFINE target already has a language-owned binding: {identity}"),
                    arguments[0].span,
                ));
            }

            let value = evaluate(&arguments[1], environment)?;
            if !matches!(value, Value::Closure(_) | Value::Builtin(_)) {
                return Err(LanguageError::new(
                    ErrorKind::Type,
                    "exact DEFINE target requires a callable closure or builtin mechanism",
                    arguments[1].span,
                ));
            }

            if !environment.bind_domain_code_slot_once(identity, value.clone()) {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("exact DEFINE target was concurrently bound: {identity}"),
                    arguments[0].span,
                ));
            }

            Ok(value)
        }
        _ => Err(LanguageError::new(
            ErrorKind::InvalidForm,
            "def expects a symbol name or exact callable DomainIdentity · def ochikuie nazvu-symvol abo tochnyi vyklychnyi DomainIdentity · def erwartet einen Symbolnamen oder eine exakte aufrufbare DomainIdentity",
            arguments[0].span,
        )),
    }
}

pub fn exact_arity(
    operator: &str,
    arguments: &[Expr],
    expected: usize,
    span: Span,
) -> Result<(), LanguageError> {
    if arguments.len() == expected {
        return Ok(());
    }
    Err(LanguageError::new(
        ErrorKind::Arity,
        format!(
            "{operator}: expected / ochikuvalosia / erwartet {expected}; received / otrymano / erhalten {}",
            arguments.len()
        ),
        span,
    ))
}

/// Перевірка арності лише за точною доменною координатою.
///
/// Вона не конструює SID8, не відкидає біти і не дозволяє однаковому
/// числовому payload у D3/D4/D5 бути однією функцією.
pub(crate) fn exact_domain_arity(
    identity: CoreDomainIdentity,
    arguments: &[Expr],
    expected: usize,
    span: Span,
) -> Result<(), LanguageError> {
    if arguments.len() == expected {
        return Ok(());
    }
    Err(LanguageError::new(
        ErrorKind::Arity,
        format!(
            "D{}:{identity}: очікувано {expected} аргументів; отримано {}",
            identity.width(),
            arguments.len()
        ),
        span,
    ))
}

pub(crate) fn quoted(expression: &Expr) -> Result<Value, LanguageError> {
    fn go(expression: &Expr, depth: u32) -> Result<Value, LanguageError> {
        if depth > crate::syntax::MAX_STRUCTURE_DEPTH {
            return Err(LanguageError::new(
                ErrorKind::Parse,
                "quoted structure exceeds reader limit · struktura perevyshchuie mezhu chytacha · zitierte Struktur überschreitet das Reader-Limit",
                Span { start: 0, end: 0 },
            ));
        }
        Ok(match &expression.kind {
            ExprKind::Number(number, exactness) => Value::Number(*number, *exactness),
            ExprKind::Rational(rational) => Value::Rational(rational.clone()),
            ExprKind::BinaryNumber(number) => Value::BinaryNumber(number.clone()),
            ExprKind::Sid(sid) => Value::Sid(*sid),
            // D3:000 is the language's canonical empty list, not a callable/data
            // identity when quoted. Preserve every other domain identity at data
            // position so exact-width residents do not collapse into names/numbers.
            ExprKind::DomainIdentity(identity) => match identity {
                crate::DomainIdentity::D3(word) if word.word().packed_bits() == 0 => Value::Nil,
                _ => Value::DomainIdentity(*identity),
            },
            ExprKind::NumericBuffer(buffer) => Value::NumericBuffer(buffer.clone()),
            ExprKind::String(value) => Value::String(value.clone()),
            ExprKind::Symbol(symbol) => Value::Symbol(symbol.clone()),
            // Слот — координата виконання, не дані: у quote-позиції його не буває
            // (розв'язувач її не чіпає), тож така поява — порушення інваріанта.
            ExprKind::Local { .. } => {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    "a lexical slot has no source datum · leksychnyi slot ne maie danykh dzherela · ein lexikalischer Slot hat kein Quelldatum",
                    expression.span,
                ));
            }
            ExprKind::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items.iter() {
                    out.push(go(item, depth + 1)?);
                }
                Value::list(out)
            }
            ExprKind::Pair(head, tail) => {
                Value::Pair(Rc::new(go(head, depth + 1)?), Rc::new(go(tail, depth + 1)?))
            }
            // Зведений виклик як дані — список із функцією СЕНС у голові.
            ExprKind::Call(sid, arguments) => {
                let mut out = Vec::with_capacity(arguments.len() + 1);
                out.push(Value::Sid(*sid));
                for argument in arguments.iter() {
                    out.push(go(argument, depth + 1)?);
                }
                Value::list(out)
            }
            ExprKind::DomainCall(identity, arguments) => {
                let mut out = Vec::with_capacity(arguments.len() + 1);
                out.push(Value::DomainIdentity((*identity).into()));
                for argument in arguments.iter() {
                    out.push(go(argument, depth + 1)?);
                }
                Value::list(out)
            }
        })
    }
    go(expression, 0)
}

// ── contract 2.1: value-level entry points (first-class builtins) ──
// Same compute as the expr-handlers above; arguments arrive
// pre-evaluated. The expr-handlers delegate here after evaluating.

pub(crate) fn car_value(value: &Value, span: Span) -> Result<Value, LanguageError> {
    match value {
        Value::Pair(ref head, _) => Ok((**head).clone()),
        _ => Err(LanguageError::new(
            ErrorKind::Type,
            "car expects a non-empty list · car ochikuie neporozhnii spysok · car erwartet eine nicht leere Liste",
            span,
        )),
    }
}

pub(crate) fn cdr_value(value: &Value, span: Span) -> Result<Value, LanguageError> {
    match value {
        Value::Pair(_, ref tail) => Ok((**tail).clone()),
        _ => Err(LanguageError::new(
            ErrorKind::Type,
            "cdr expects a non-empty list · cdr ochikuie neporozhnii spysok · cdr erwartet eine nicht leere Liste",
            span,
        )),
    }
}

pub(crate) fn cons_values(
    head: Value,
    tail: Value,
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    if environment.try_alloc_cons().is_err() {
        return Err(LanguageError::new(
            ErrorKind::OutOfMemory,
            "cons: resource limit reached · cons: dosiahnuto mezhi resursu · cons: Ressourcengrenze erreicht",
            span,
        ));
    }
    Ok(Value::Pair(std::rc::Rc::new(head), std::rc::Rc::new(tail)))
}

pub(crate) fn eq_values(left: Value, right: Value, span: Span) -> Result<Value, LanguageError> {
    if !left.is_atom() || !right.is_atom() {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "eq expects two atoms · eq ochikuie dva atomy · eq erwartet zwei Atome",
            span,
        ));
    }
    Ok(Value::predicate_bit(left == right))
}

#[cfg(test)]
mod exact_d4_define_arity_tests {
    use super::*;

    fn expression() -> Expr {
        Expr {
            kind: ExprKind::Symbol(Rc::from("ім'я")),
            span: Span::default(),
        }
    }

    #[test]
    fn quote_of_canonical_d3_empty_coordinate_is_structural_nil() {
        let empty = Expr {
            kind: ExprKind::DomainIdentity(crate::DomainIdentity::D3(
                crate::Bija3::from_word(crate::Bit3::new(0).unwrap()),
            )),
            span: Span::default(),
        };
        assert!(matches!(quoted(&empty).unwrap(), Value::Nil));
    }

    #[test]
    fn quote_preserves_nonempty_d3_domain_identity() {
        let quote = crate::Bija3::from_word(crate::Bit3::new(0b001).unwrap());
        let expression = Expr {
            kind: ExprKind::DomainIdentity(crate::DomainIdentity::D3(quote)),
            span: Span::default(),
        };
        assert!(matches!(
            quoted(&expression).unwrap(),
            Value::DomainIdentity(crate::DomainIdentity::D3(value)) if value == quote
        ));
    }

    #[test]
    fn визначення_діагностує_тільки_точну_d4_координату() {
        let environment = Environment::root();
        for count in [0usize, 1, 3] {
            let arguments = vec![expression(); count];
            let error = evaluate_definition(&arguments, &environment, Span::default())
                .expect_err("хибна арність має бути відхилена раніше за виконання");
            assert_eq!(error.kind, ErrorKind::Arity);
            assert!(error.message.contains("D4:0011"), "{error:?}");
            assert!(!error.message.contains("00001011"), "{error:?}");
        }
    }

    #[test]
    fn правильна_арність_і_неоднакові_ширини_доменів() {
        let d4 = CoreDomainIdentity::D4(
            CoreD4::from_word(Bit4::new(0b0011).unwrap()),
        );
        let two = vec![expression(), expression()];
        assert!(exact_domain_arity(d4, &two, 2, Span::default()).is_ok());
        let wrong = CoreDomainIdentity::D3(
            crate::Bija3::from_word(crate::Bit3::new(0b011).unwrap()),
        );
        let error = exact_domain_arity(wrong, &[], 1, Span::default()).unwrap_err();
        assert!(error.message.contains("D3:011"), "{error:?}");
        assert!(!error.message.contains("D4:0011"), "{error:?}");
    }
}
