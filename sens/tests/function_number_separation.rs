use sens::{eval_program, parse, ErrorKind, Exactness, ExprKind, Session, Value};

#[test]
fn sens_function_value_is_not_a_number_in_arithmetic() {
    let error = eval_program("(00001100 00000010 1)", &mut Session::default())
        .expect_err("a SENS function value must not participate in arithmetic");

    assert_eq!(error.kind, ErrorKind::Type);
}

#[test]
fn computed_number_with_matching_bits_does_not_become_callable() {
    // 1 + 1 = numeric value 2. Function 00000010 has the same packed
    // magnitude, but the domains are orthogonal: the computed number must
    // remain a number and therefore cannot become callable.
    let error = eval_program("((00001100 1 1) 1)", &mut Session::default())
        .expect_err("numeric value 2 must not become function 00000010");

    assert_eq!(error.kind, ErrorKind::Type);
}

#[test]
fn number_and_sens_function_are_distinct_even_when_magnitudes_match() {
    let parsed = parse("00001100").expect("exact SENS source must parse");
    let [expr] = parsed.as_slice() else {
        panic!("expected one parsed expression");
    };
    assert!(
        matches!(expr.kind, ExprKind::Sid(_)),
        "bare exact 8-bit source must remain a SENS function value"
    );

    let number = Value::Number(12.0, Exactness::Exact);
    let function = eval_program("00001100", &mut Session::default())
        .expect("bare SENS function evaluates as a first-class function value")
        .value;
    assert_ne!(
        number, function,
        "numeric 12 and function 00001100 must never share identity"
    );
}
