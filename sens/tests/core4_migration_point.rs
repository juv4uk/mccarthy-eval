use sens::{eval_program, load_core_library, Session, Value};

fn migrated_core() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("migrated Core4 must load");
    session
}

#[test]
fn core4_point_witness_crosses_predicates_strings_macros_and_fresh_symbols() {
    let mut session = migrated_core();

    for (source, expected) in [
        ("(equal? (quote (a b)) (quote (a b)))", true),
        ("(equal? (quote (a b)) (quote (a c)))", false),
        ("(member? (quote b) (quote (a b c)))", true),
        ("(member? (quote z) (quote (a b c)))", false),
        ("(null? (quote ()))", true),
        ("(null? (quote (a)))", false),
        ("(string<? \"a\" \"b\")", true),
    ] {
        let value = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("{source} failed: {e:?}"))
            .value;
        assert_eq!(value.as_predicate_bit(), Some(expected), "{source} -> {value}");
    }

    for (source, expected) in [
        ("(assoc (quote b) (quote ((a . 1) (b . 2))))", "(b . 2)"),
        ("(assoc (quote z) (quote ((a . 1) (b . 2))))", "()"),
    ] {
        let value = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("{source} failed: {e:?}"))
            .value;
        assert_eq!(value.to_string(), expected, "{source}");
    }

    for (source, expected) in [
        ("(let ((x 7)) x)", "7"),
        ("(let* ((x 7) (y x)) y)", "7"),
        ("(-> 3 (list 4))", "(3 4)"),
        ("(->> 3 (list 4))", "(4 3)"),
    ] {
        let value = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("{source} failed: {e:?}"))
            .value;
        assert_eq!(value.to_string(), expected, "{source}");
    }

    let fresh = eval_program("(gensym \"core4-point-\")", &mut session)
        .expect("GENSYM mechanism boundary")
        .value;
    assert!(matches!(fresh, Value::Symbol(_)));
}
