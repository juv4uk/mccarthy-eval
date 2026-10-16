//! Research witness for #3937.
//!
//! This proves the current value-level structure<->text law only.
//! It does not ratify D8 coordinates.

use sens::{eval_program, Session, Value};

fn eval(source: &str) -> Value {
    eval_program(source, &mut Session::default())
        .expect("research witness source should evaluate")
        .value
}

#[test]
fn read_does_not_evaluate_the_parsed_form() {
    let datum = eval(r#"(read "(+ 1 2)")"#);
    assert_eq!(
        datum,
        Value::list([
            Value::Symbol("+".into()),
            Value::Number(1.0, sens::Exactness::Exact),
            Value::Number(2.0, sens::Exactness::Exact),
        ])
    );
}

#[test]
fn read_after_canonical_write_is_identity_on_basic_readable_values() {
    let cases = [
        "(quote ())",
        "(quote alpha)",
        "42",
        r#""radio""#,
        "(quote (a b c))",
        "(quote (a b . c))",
        "(quote ((a . b) (c d)))",
    ];

    for source in cases {
        let expected = eval(source);
        let round_trip = eval(&format!("(read (write-to-string {source}))"));
        assert_eq!(round_trip, expected, "round-trip failed for {source}");
    }
}

#[test]
fn write_after_read_is_a_canonicalization_not_source_identity() {
    let canonical = eval(r#"(write-to-string (read "(a . (b . ()))"))"#);
    assert_eq!(canonical, Value::String("(a b)".into()));
}
