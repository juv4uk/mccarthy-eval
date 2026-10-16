use sens::{
    eval_program, parse, parse_json, ErrorKind, Exactness, ExprKind, NumericBuffer, Rational,
    Session, Value,
};

const TWO_POW_53: i64 = 9_007_199_254_740_992;
const TWO_POW_53_PLUS_ONE: &str = "9007199254740993";

fn eval(source: &str) -> Value {
    eval_program(source, &mut Session::default())
        .unwrap_or_else(|error| panic!("{source:?} failed: {error}"))
        .value
}

#[test]
fn compact_exact_integer_boundary_is_exactly_two_pow_53() {
    let cases = [
        ("9007199254740991", true),
        ("9007199254740992", true),
        ("9007199254740993", false),
        ("-9007199254740991", true),
        ("-9007199254740992", true),
        ("-9007199254740993", false),
    ];

    for (source, compact) in cases {
        let form = parse(source).expect("boundary integer must parse").remove(0);
        match (compact, form.kind) {
            (true, ExprKind::Number(_, Exactness::Exact)) => {}
            (false, ExprKind::Rational(_)) => {}
            (expected_compact, other) => panic!(
                "{source}: expected compact={expected_compact}, got representation {other:?}"
            ),
        }
    }

    let at_edge = Rational::from_literal("9007199254740992", "1").unwrap();
    assert_eq!(at_edge.as_precise_i64(), Some(TWO_POW_53));
    let outside = Rational::from_literal(TWO_POW_53_PLUS_ONE, "1").unwrap();
    assert_eq!(outside.as_precise_i64(), None);
}

#[test]
fn fast_path_product_past_f64_exact_range_never_silently_rounds() {
    // Both operands enter the small-i64 fast path. Their exact product is an
    // odd i64 above 2^53, so representing it as f64 would necessarily round.
    const EXPECTED: &str = "9000000006000000001";
    let product = eval("(* 3000000001 3000000001)");

    assert_eq!(product.to_string(), EXPECTED);
    let Value::Rational(rational) = &product else {
        panic!("an exact integer above the f64 exact range must stay Rational");
    };
    assert_eq!(rational.to_string(), EXPECTED);
}

#[test]
fn arithmetic_crosses_compact_boundary_without_changing_exact_value() {
    let cases = [
        ("(+ 9007199254740992 1)", "9007199254740993"),
        ("(- -9007199254740992 1)", "-9007199254740993"),
        ("(+ 9007199254740993 7)", "9007199254741000"),
        ("(* 9007199254740993 3)", "27021597764222979"),
    ];

    for (source, expected) in cases {
        assert_eq!(eval(source).to_string(), expected, "source: {source}");
    }
}

#[test]
fn exact_decimal_and_exponent_literals_never_enter_binary_float_semantics() {
    let cases = [
        ("0.1", "1/10"),
        ("1,25", "5/4"),
        ("1e-3", "1/1000"),
        ("1.25e2", "125"),
        ("1e20", "100000000000000000000"),
    ];

    for (source, expected) in cases {
        assert_eq!(eval(source).to_string(), expected, "literal: {source}");
    }

    assert_eq!(eval("(* 1e-100 1e100)").to_string(), "1");
    let ten_pow_200 = format!("1{}", "0".repeat(200));
    assert_eq!(eval("(/ 1e100 1e-100)").to_string(), ten_pow_200);
}

#[test]
fn exact_division_reduces_without_losing_large_magnitude() {
    assert_eq!(
        eval("(/ 9007199254740993 2)").to_string(),
        "9007199254740993/2"
    );
    assert_eq!(
        eval("(/ 9007199254740993 3)").to_string(),
        "3002399751580331"
    );
    assert_eq!(eval("(/ 1 10)").to_string(), "1/10");
}

#[test]
fn print_read_eval_round_trip_preserves_large_exact_integer() {
    let source = "(eval (read (write-to-string (+ 9007199254740992 1))))";
    let value = eval(source);
    assert_eq!(value.to_string(), TWO_POW_53_PLUS_ONE);
    assert!(matches!(value, Value::Rational(_)));
}

#[test]
fn json_integer_tokens_share_the_same_exact_compression_boundary() {
    let compact = parse_json("9007199254740992").expect("JSON integer at 2^53");
    assert!(matches!(compact, Value::Number(_, Exactness::Exact)));
    assert_eq!(compact.to_string(), "9007199254740992");

    let one_past = parse_json(TWO_POW_53_PLUS_ONE).expect("JSON integer above 2^53");
    assert!(matches!(one_past, Value::Rational(_)));
    assert_eq!(one_past.to_string(), TWO_POW_53_PLUS_ONE);

    let arbitrary = parse_json("123456789012345678901234567890")
        .expect("JSON integer should use arbitrary-precision exact path");
    assert!(matches!(arbitrary, Value::Rational(_)));
    assert_eq!(arbitrary.to_string(), "123456789012345678901234567890");

    let decimal = parse_json("0.1").expect("JSON decimal stays the explicit inexact boundary");
    assert!(matches!(decimal, Value::Number(_, Exactness::Inexact)));
}

#[test]
fn parser_resource_limit_stays_a_named_numeric_failure() {
    for source in ["1e10001", "1e-10001"] {
        let error = parse(source).expect_err("one past the exponent cap must fail");
        assert_eq!(error.kind, ErrorKind::NumericOverflow, "source: {source}");
    }
}

#[test]
fn typed_buffer_conversion_is_explicit_narrowing_not_exact_relabeling() {
    let form = parse("#f32(0.1 1/3)")
        .expect("finite exact inputs may be explicitly narrowed to f32")
        .remove(0);
    let ExprKind::NumericBuffer(NumericBuffer::F32(values)) = form.kind else {
        panic!("expected an f32 numeric buffer");
    };
    assert_eq!(values[0].to_bits(), 0.1f32.to_bits());
    assert_eq!(values[1].to_bits(), (1.0f32 / 3.0).to_bits());

    let error = parse("#i32(2147483648)").expect_err("i32 narrowing must fail named");
    assert_eq!(error.kind, ErrorKind::NumericOverflow);
}
