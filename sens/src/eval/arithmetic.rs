//! Current exact D24+ mathematical operations. D1–D10 words, host numeric
//! literals, legacy Number/Rational and unqualified BinaryNumber are never
//! admitted arithmetic operands. The legacy value projection helper remains
//! only for independent non-mathematical readers (e.g. JSON).
use crate::domain_numeric;
use crate::{BinaryNumber, Environment, ErrorKind, Exactness, LanguageError, Rational, Span, Value};

/// Legacy data/presentation bridge, NOT a mathematical operand admission.
/// Existing JSON decoding uses this conversion independently of Core math.
pub(crate) fn exact_value(value: Rational) -> Value {
    match value.as_precise_i64() {
        Some(n) => Value::Number(n as f64, Exactness::Exact),
        None => Value::Rational(value),
    }
}

fn arithmetic_overflow(span: Span) -> LanguageError {
    LanguageError::new(
        ErrorKind::NumericOverflow,
        "exact arithmetic overflow · perepovnennia tochnoi aryfmetyky · Überlauf der exakten Arithmetik",
        span,
    )
}

fn division_error(span: Span) -> LanguageError {
    LanguageError::new(
        ErrorKind::DivisionByZero,
        "division by zero · dilennia na nul · Division durch null",
        span,
    )
}

/// Only explicit numeric D24/D48/... values admit Core mathematical operations.
/// Reader literals and host/Rust Number remain representable as data, not math.
fn require_math_domain(values: &[Value], span: Span) -> Result<(), LanguageError> {
    if values.is_empty() || values.iter().any(|v| !domain_numeric::is_domain_value(v)) {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "math requires explicit D24+ operands; D1-D10 bit words and unqualified numbers are not arithmetic values",
            span,
        ));
    }
    Ok(())
}

fn exact_domain_result_width(result: &Value, span: Span) -> Result<usize, LanguageError> {
    match result {
        Value::BinaryNumber(number) => Ok(number.width()),
        Value::NumericComposition(composition) => {
            let (_, numerator, denominator) = composition.as_parts();
            Ok(numerator.width().max(denominator.map_or(0, BinaryNumber::width)))
        }
        _ => Err(LanguageError::new(ErrorKind::Type, "numeric composite invariant", span)),
    }
}

fn check_domain_result_limit(
    result: &Value,
    environment: &Environment,
    span: Span,
) -> Result<(), LanguageError> {
    let bit_length = exact_domain_result_width(result, span)?;
    if environment.numeric_bit_limit().is_some_and(|limit| bit_length > limit) {
        return Err(arithmetic_overflow(span));
    }
    Ok(())
}

/// No literal/legacy fallback: one admitted D24+ arithmetic execution path.
pub(super) fn arithmetic_on_values(
    operator: &str,
    values: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    require_math_domain(values, span)?;
    let result = match values {
        [single] if operator == "-" => domain_numeric::unary("-", single),
        [left, right] => domain_numeric::compute(operator, left, right),
        _ => None,
    }
    .ok_or_else(|| {
        LanguageError::new(
            ErrorKind::Type,
            "numeric Dn composition requires same-domain operands and a supported operation",
            span,
        )
    })?;
    check_domain_result_limit(&result, environment, span)?;
    Ok(result)
}

/// D24+ reciprocal and exact rational division, never legacy float/Rational.
pub(super) fn division_on_values(
    values: &[Value],
    argument_count: usize,
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    if values.is_empty() {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            "/ expects at least one argument · / ochikuie shchonaimenshe odyn arhument · / erwartet mindestens ein Argument",
            span,
        ));
    }
    require_math_domain(values, span)?;
    // An arithmetic Dn / D(2n) operand pair is invalid *before*
    // checking for division by zero (which needs an admitted division).
    let admitted_operands = match values {
        [single] => domain_numeric::is_domain_value(single),
        // Domain admission is an exact-width identity check, not a
        // rational equality query. Comparing fractions here would multiply
        // potentially huge numerators before the actual division even starts.
        [lhs, rhs] if argument_count == 2 =>
            domain_numeric::shared_numeric_domain_width(lhs, rhs).is_some(),
        _ => false,
    };
    if !admitted_operands {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "numeric division requires admitted same-Dn values",
            span,
        ));
    }
    let denominator_is_zero = match values {
        [single] => matches!(single, Value::BinaryNumber(n) if n.is_zero()),
        [_, rhs] => matches!(rhs, Value::BinaryNumber(n) if n.is_zero()),
        _ => false,
    };
    if denominator_is_zero {
        return Err(division_error(span));
    }
    let result = match values {
        [single] => domain_numeric::unary("/", single),
        [lhs, rhs] if argument_count == 2 => domain_numeric::compute("/", lhs, rhs),
        _ => None,
    }
    .ok_or_else(|| {
        LanguageError::new(
            ErrorKind::Type,
            "Dn rational division requires exactly two same-domain inputs or unary reciprocal",
            span,
        )
    })?;
    check_domain_result_limit(&result, environment, span)?;
    Ok(result)
}

/// Numeric order is D24+ only. D3 EQ remains a distinct identity law.
pub(super) fn comparison_on_values(
    operator: &str,
    values: &[Value],
    span: Span,
) -> Result<Value, LanguageError> {
    require_math_domain(values, span)?;
    let holds = match values {
        [left, right] => domain_numeric::compare(operator, left, right),
        _ => None,
    }
    .ok_or_else(|| {
        LanguageError::new(
            ErrorKind::Type,
            "Dn comparison requires exactly two same-domain numeric values",
            span,
        )
    })?;
    Ok(Value::predicate_bit(holds))
}

#[cfg(test)]
mod qualified_numeric_domain_value_tests {
    use super::*;

    #[test]
    fn binary_code_domains_and_unqualified_numbers_are_never_math_operands() {
        let env = Environment::root();
        let words = [
            crate::BinarySourceWord::W1(crate::Bit1::new(1).unwrap()),
            crate::BinarySourceWord::W2(crate::Bit2::new(1).unwrap()),
            crate::BinarySourceWord::W3(crate::Bit3::new(1).unwrap()),
            crate::BinarySourceWord::W4(crate::Bit4::new(1).unwrap()),
            crate::BinarySourceWord::W5(crate::Bit5::new(1).unwrap()),
            crate::BinarySourceWord::W6(crate::Bit6::new(1).unwrap()),
            crate::BinarySourceWord::W7(crate::Bit7::new(1).unwrap()),
            crate::BinarySourceWord::W8(crate::Bit8::new(1).unwrap()),
            crate::BinarySourceWord::W9(crate::Bit9::new(1).unwrap()),
        ];
        for word in words {
            let identity = crate::DomainIdentity::from_source_word(word);
            let value = Value::DomainIdentity(identity);
            for op in ["+", "-", "*"] {
                assert_eq!(
                    arithmetic_on_values(op, &[value.clone(), value.clone()], &env, Span::default())
                        .unwrap_err().kind,
                    ErrorKind::Type,
                    "D{} is not an arithmetic operand", identity.width()
                );
            }
            assert_eq!(division_on_values(&[value.clone(), value.clone()], 2, &env, Span::default())
                .unwrap_err().kind, ErrorKind::Type);
            for op in ["=", "<", ">"] {
                assert_eq!(comparison_on_values(op, &[value.clone(), value.clone()], Span::default())
                    .unwrap_err().kind, ErrorKind::Type);
            }
        }
        for value in [
            Value::Number(1.0, Exactness::Exact),
            Value::Rational(Rational::integer(1)),
            Value::BinaryNumber(BinaryNumber::one()),
        ] {
            assert_eq!(arithmetic_on_values("+", &[value.clone(), value.clone()], &env, Span::default())
                .unwrap_err().kind, ErrorKind::Type);
            assert_eq!(division_on_values(&[value.clone(), value.clone()], 2, &env, Span::default())
                .unwrap_err().kind, ErrorKind::Type);
            assert_eq!(comparison_on_values("=", &[value.clone(), value], Span::default())
                .unwrap_err().kind, ErrorKind::Type);
        }
        assert_eq!(
            arithmetic_on_values("+", &[], &env, Span::default()).unwrap_err().kind,
            ErrorKind::Type
        );
    }

    fn in_domain(width: usize, bits: Vec<u8>) -> Value {
        Value::BinaryNumber(
            BinaryNumber::from_numeric_domain_bits(width, &bits)
                .expect("exact numeric domain bits"),
        )
    }

    #[test]
    fn core_runtime_adds_d24_values_to_d48_value_and_retains_domain_identity() {
        let max = in_domain(24, vec![1; 24]);
        let env = Environment::root();
        let sum = arithmetic_on_values("+", &[max.clone(), max.clone()], &env, Span::default())
            .expect("same-domain natural ADD");
        let Value::BinaryNumber(ref sum_bits) = sum else {
            panic!("must return an exact domain-qualified numeric Value")
        };
        assert_eq!(sum_bits.numeric_domain_width(), Some(48));
        let full = sum_bits.exact_numeric_domain_bits().unwrap();
        assert_eq!(full.len(), 48);
        assert_eq!(&full[..23], &[0; 23]);
        assert_eq!(full[47], 0);

        let zero = in_domain(24, vec![0; 24]);
        let result = arithmetic_on_values("+", &[zero.clone(), zero], &env, Span::default())
            .unwrap();
        let Value::BinaryNumber(ref same) = result else { panic!("D24 result") };
        assert_eq!(same.numeric_domain_width(), Some(24));
        assert_eq!(same.exact_numeric_domain_bits(), Some(vec![0; 24]));
    }

    #[test]
    fn source_expression_uses_binary_domain_values_without_numeric_literals() {
        // Only the admitted D24 payloads are supplied directly as values.
        // This is the real evaluator path through a named environment binding.
        let mut session = crate::Session::default();
        let max = in_domain(24, vec![1; 24]);
        session.environment.define("a", max.clone());
        session.environment.define("b", max);
        let result = crate::eval_program("(+ a b)", &mut session)
            .expect("Core addition over domain-qualified values").value;
        let Value::BinaryNumber(ref value) = result else {
            panic!("domain-qualified Value expected")
        };
        assert_eq!(value.numeric_domain_width(), Some(48));
        assert_eq!(value.exact_numeric_domain_bits().unwrap().len(), 48);
    }

    #[test]
    fn core_runtime_exact_d24_natural_mul_sub_div_without_literals() {
        let mut session = crate::Session::default();
        let make = |number: u32| {
            let mut bits = vec![0u8;24];
            for i in 0..24 { bits[23-i] = ((number>>i)&1) as u8; }
            in_domain(24,bits)
        };
        session.environment.define("a", make(6));
        session.environment.define("b", make(2));
        for (expr, expected) in [
            ("(* a b)", 12),
            ("(- a b)", 4),
            ("(/ a b)", 3),
        ] {
            let result = crate::eval_program(expr, &mut session)
                .expect("same-D24 natural arithmetic").value;
            let Value::BinaryNumber(ref bits) = result else { panic!("numeric D24 Value") };
            assert_eq!(bits.numeric_domain_width(), Some(24), "{expr}");
            assert_eq!(bits.to_decimal_string(), expected.to_string(), "{expr}");
        }
        session.environment.define("a", make(1));
        session.environment.define("b", make(3));
        let minus = crate::eval_program("(- a b)", &mut session).unwrap().value;
        assert_eq!(minus.to_string(), "-2");
        let fraction = crate::eval_program("(/ a b)", &mut session).unwrap().value;
        assert_eq!(fraction.to_string(), "1/3");
        session.environment.define("negative", minus);
        session.environment.define("fraction", fraction);
        assert_eq!(crate::eval_program("(+ negative b)", &mut session).unwrap().value, make(1));
        assert_eq!(crate::eval_program("(* fraction b)", &mut session).unwrap().value, make(1));
        session.environment.define("b", make(0));
        let err = crate::eval_program("(/ a b)", &mut session)
            .expect_err("divide by zero must fail");
        assert_eq!(err.kind, ErrorKind::DivisionByZero);
    }

    /// One focused Core execution witness. The mathematical comparison is
    /// performed by the exact D5 SENS functions, not by a host Fraction/f64
    /// oracle; Rust only inspects the returned D1 predicate and named failures.
    /// D24 operands are injected as exact domain-qualified values because the
    /// physical source-to-D24 reader boundary remains a separate task.
    #[test]
    fn binary_d5_exact_q_axiom_witness_with_single_input_mutation() {
        let mut session = crate::Session::default();
        let qualified = |width: usize, bits: &str| -> Value {
            assert!(bits.len() <= width && !bits.is_empty());
            let mut word = vec![0u8; width - bits.len()];
            word.extend(bits.bytes().map(|bit| match bit {
                b'0' => 0,
                b'1' => 1,
                _ => panic!("expected exact binary magnitude"),
            }));
            Value::BinaryNumber(
                BinaryNumber::from_numeric_domain_bits(width, &word)
                    .expect("full-width admitted Dn input"),
            )
        };
        for (name, bits) in [
            ("a", "1"), ("b", "10"), ("c", "11"), ("f", "110"),
            ("g", "111"), ("z", "0"),
        ] {
            session.environment.define(name, qualified(24, bits));
        }
        // Same magnitude as D24:a, but a different domain identity.
        session.environment.define("w", qualified(48, "1"));
        session.environment.define("u", Value::BinaryNumber(BinaryNumber::one()));

        // D5 10111 = quotient, D5 01010 = sum, D5 11010/11011
        // = less/greater. The two D1:NO results prove exact equality
        // of 1/3 + 1/6 and 1/2 under SENS's admitted total order.
        // The one-input mutation 1/6 -> 1/7 MUST change the less verdict.
        let cases = [
            ("equal-not-less", "(11010 (01010 (10111 a c) (10111 a f)) (10111 a b))", false),
            ("equal-not-greater", "(11011 (01010 (10111 a c) (10111 a f)) (10111 a b))", false),
            ("mutated-less", "(11010 (01010 (10111 a c) (10111 a g)) (10111 a b))", true),
        ];
        for (id, source, expected) in cases {
            let parsed = crate::parse_mixed_exact_domain(source)
                .unwrap_or_else(|err| panic!("id={id} layer=parse source={source} actual={err}"));
            let actual = crate::eval_parsed_expressions(&parsed, &mut session)
                .unwrap_or_else(|err| panic!("id={id} layer=eval source={source} actual={err}"))
                .value.as_predicate_bit();
            assert_eq!(
                actual,
                Some(expected),
                "REGRESSION id={id} layer=eval source={source} expected=D1:{} actual={actual:?} reproduce=cargo test --locked -p sens --lib binary_d5_exact_q_axiom_witness_with_single_input_mutation -- --nocapture",
                u8::from(expected),
            );
        }

        // A source-level exact binary D5 call must not grant arithmetic
        // admission to a zero divisor, a different Dn or a bare Number.
        for (id, source, kind) in [
            ("denominator-zero", "(10111 a z)", ErrorKind::DivisionByZero),
            ("mixed-d24-d48", "(10111 a w)", ErrorKind::Type),
            ("unqualified-number", "(10111 a u)", ErrorKind::Type),
        ] {
            let parsed = crate::parse_mixed_exact_domain(source)
                .unwrap_or_else(|err| panic!("id={id} layer=parse source={source} actual={err}"));
            let error = crate::eval_parsed_expressions(&parsed, &mut session)
                .expect_err("negative D5 witness must fail closed");
            assert_eq!(
                error.kind, kind,
                "REGRESSION id={id} layer=eval source={source} expected={kind:?} actual={error:?} reproduce=cargo test --locked -p sens --lib binary_d5_exact_q_axiom_witness_with_single_input_mutation -- --nocapture",
            );
        }
    }

    #[test]
    fn core_runtime_d24_full_multiplication_produces_exact_d48() {
        let env = Environment::root();
        let max = in_domain(24, vec![1;24]);
        let result = arithmetic_on_values("*", &[max.clone(),max], &env, Span::default()).unwrap();
        let Value::BinaryNumber(ref bits) = result else { panic!("numeric D48 Value") };
        assert_eq!(bits.numeric_domain_width(), Some(48));
        assert_eq!(bits.width(), 48);
        assert_eq!(bits.exact_numeric_domain_bits().unwrap().len(), 48);
    }

    #[test]
    fn core_runtime_rejects_unadmitted_numeric_domain_operations() {
        let env = Environment::root();
        let d24 = in_domain(24, vec![0; 24]);
        let d48 = in_domain(48, vec![0; 48]);
        for values in [
            vec![d24.clone(), d48.clone()],
            vec![d24.clone(), Value::BinaryNumber(BinaryNumber::one())],
        ] {
            let e = arithmetic_on_values("+", &values, &env, Span::default()).unwrap_err();
            assert_eq!(e.kind, ErrorKind::Type);
            let cmp = comparison_on_values("=", &values, Span::default()).unwrap_err();
            assert_eq!(cmp.kind, ErrorKind::Type);
        }
        for op in ["*", "-", "+"] {
            let e = arithmetic_on_values(op, &[d24.clone(), d48.clone()], &env, Span::default()).unwrap_err();
            assert_eq!(e.kind, ErrorKind::Type);
        }
        let e = division_on_values(&[d24.clone(), d48.clone()], 2, &env, Span::default()).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Type);
        let e = arithmetic_on_values("+", &[d24], &env, Span::default()).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Type);
    }
}
