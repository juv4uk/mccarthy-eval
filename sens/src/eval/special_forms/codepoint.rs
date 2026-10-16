use crate::{ErrorKind, Exactness, LanguageError, Span, Value};
use std::rc::Rc;

/// Minimal runtime bridge from an exact Unicode scalar value to the language's
/// immutable string representation. Unicode/UTF-8 decoding policy stays in
/// Lisp; this function only materializes one already-interpreted scalar.
pub(crate) fn codepoint_to_string_values(
    arguments: &[Value],
    span: Span,
) -> Result<Value, LanguageError> {
    if arguments.len() != 1 {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            "codepoint->string expects exactly 1 argument(s)",
            span,
        ));
    }
    let scalar = exact_scalar_value(&arguments[0], span)?;
    let character = char::from_u32(scalar).ok_or_else(|| {
        LanguageError::new(
            ErrorKind::Type,
            "codepoint->string expects a Unicode scalar value · codepoint->string ochikuie skaliarne znachennia Unicode · codepoint->string erwartet einen Unicode-Skalarwert",
            span,
        )
    })?;
    Ok(Value::String(Rc::from(character.to_string().as_str())))
}

/// Minimal inverse bridge for UTF-8 encoding owned by Lisp. The runtime only
/// exposes the scalar value of exactly one already-materialized character;
/// byte encoding, validation policy, and transport meaning remain in Lisp.
pub(crate) fn string_to_codepoint_values(
    arguments: &[Value],
    span: Span,
) -> Result<Value, LanguageError> {
    if arguments.len() != 1 {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            "string->codepoint expects exactly 1 argument(s)",
            span,
        ));
    }
    let Value::String(text) = &arguments[0] else {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "string->codepoint expects a one-character string · string->codepoint ochikuie riadok z odnoho symvolu · string->codepoint erwartet eine Zeichenkette mit genau einem Zeichen",
            span,
        ));
    };

    let mut characters = text.chars();
    let Some(character) = characters.next() else {
        return Err(invalid_character_string(span));
    };
    if characters.next().is_some() {
        return Err(invalid_character_string(span));
    }

    Ok(Value::Number(character as u32 as f64, Exactness::Exact))
}

fn exact_scalar_value(value: &Value, span: Span) -> Result<u32, LanguageError> {
    let integer = match value {
        Value::Number(number, Exactness::Exact)
            if number.is_finite() && number.fract() == 0.0 =>
        {
            if *number < 0.0 || *number > 0x10ffff as f64 {
                return Err(invalid_scalar(span));
            }
            *number as u32
        }
        Value::Rational(rational) if rational.is_integer() => {
            let number = rational.as_precise_i64().ok_or_else(|| invalid_scalar(span))?;
            u32::try_from(number).map_err(|_| invalid_scalar(span))?
        }
        _ => return Err(invalid_scalar(span)),
    };

    if integer > 0x10ffff || (0xd800..=0xdfff).contains(&integer) {
        return Err(invalid_scalar(span));
    }
    Ok(integer)
}

fn invalid_scalar(span: Span) -> LanguageError {
    LanguageError::new(
        ErrorKind::Type,
        "codepoint->string expects an exact Unicode scalar integer (0..0x10FFFF excluding surrogates) · codepoint->string ochikuie tochne tsile skaliarne znachennia Unicode · codepoint->string erwartet eine exakte Unicode-Skalarzahl",
        span,
    )
}

fn invalid_character_string(span: Span) -> LanguageError {
    LanguageError::new(
        ErrorKind::Type,
        "string->codepoint expects exactly one Unicode scalar character · string->codepoint ochikuie rivno odyn skaliarnyi symvol Unicode · string->codepoint erwartet genau ein Unicode-Skalarzeichen",
        span,
    )
}
