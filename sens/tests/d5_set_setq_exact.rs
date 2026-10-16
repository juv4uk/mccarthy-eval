use sens::{
    eval_parsed_expressions, parse, Bit4, Bit5, CoreD4, CoreD5, DomainIdentity, Environment,
    Exactness, Expr, ExprKind, LanguageError, Session, Value,
};
use std::rc::Rc;

fn exact_form(width: u8, bits: u8, args: &str) -> Expr {
    let mut parsed = parse(&format!("(__probe__ {args})")).expect("probe parse");
    let mut form = parsed.remove(0);
    let ExprKind::List(items) = form.kind else {
        panic!("probe wrapper must be list");
    };
    let mut items = items.to_vec();
    let identity = match width {
        4 => DomainIdentity::D4(CoreD4::from_word(Bit4::new(bits).unwrap())),
        5 => DomainIdentity::D5(CoreD5::from_word(Bit5::new(bits).unwrap())),
        _ => panic!("unsupported width"),
    };
    items[0] = Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: items[0].span,
    };
    form.kind = ExprKind::List(items.into());
    form
}

fn run_exact(env: &Environment, width: u8, bits: u8, args: &str) -> Result<Value, LanguageError> {
    let form = exact_form(width, bits, args);
    let mut session = Session {
        environment: env.clone(),
    };
    eval_parsed_expressions(&[form], &mut session).map(|result| result.value)
}

fn exact_number(n: f64) -> Value {
    Value::Number(n, Exactness::Exact)
}

#[test]
fn exact_d5_setq_updates_existing_binding_and_returns_value() {
    let env = Environment::root();
    env.define("x", exact_number(1.0));

    let result = run_exact(&env, 5, 0b00111, "x 2").expect("D5 SETQ");
    assert_eq!(result, exact_number(2.0));
    assert_eq!(env.get("x"), Some(exact_number(2.0)));
}

#[test]
fn exact_d5_set_evaluates_target_while_setq_keeps_it_literal() {
    let env = Environment::root();
    env.define("x", exact_number(1.0));
    env.define("target", Value::Symbol(Rc::from("x")));

    let result = run_exact(&env, 5, 0b00110, "target 3").expect("D5 SET");
    assert_eq!(result, exact_number(3.0));
    assert_eq!(env.get("x"), Some(exact_number(3.0)));
    assert_eq!(env.get("target"), Some(Value::Symbol(Rc::from("x"))));

    let result = run_exact(&env, 5, 0b00111, "target 4").expect("D5 SETQ");
    assert_eq!(result, exact_number(4.0));
    assert_eq!(env.get("target"), Some(exact_number(4.0)));
    assert_eq!(env.get("x"), Some(exact_number(3.0)));
}

#[test]
fn exact_d5_setq_updates_nearest_shared_location() {
    let root = Environment::root();
    root.define("x", exact_number(1.0));
    let child = root.child();
    child.define("x", exact_number(2.0));
    let observer = child.child();

    run_exact(&observer, 5, 0b00111, "x 7").expect("D5 SETQ");
    assert_eq!(observer.get("x"), Some(exact_number(7.0)));
    assert_eq!(child.get("x"), Some(exact_number(7.0)));
    assert_eq!(root.get("x"), Some(exact_number(1.0)));
}

#[test]
fn exact_d5_set_family_fails_closed_on_missing_binding() {
    let env = Environment::root();

    let err = run_exact(&env, 5, 0b00111, "missing 9")
        .expect_err("D5 SETQ must not create missing binding");
    assert!(format!("{err:?}").contains("UnknownSymbol"));
    assert_eq!(env.get("missing"), None);

    let err = run_exact(&env, 5, 0b00110, "(quote missing) 9")
        .expect_err("D5 SET must not create missing binding");
    assert!(format!("{err:?}").contains("UnknownSymbol"));
    assert_eq!(env.get("missing"), None);
}

#[test]
fn d4_define_and_d5_setq_do_not_collapse() {
    let env = Environment::root();

    let d5_err = run_exact(&env, 5, 0b00111, "fresh 5")
        .expect_err("D5 SETQ must fail on missing binding");
    assert!(format!("{d5_err:?}").contains("UnknownSymbol"));
    assert_eq!(env.get("fresh"), None);

    let result = run_exact(&env, 4, 0b0011, "fresh 5").expect("D4 DEFINE");
    assert_eq!(result, exact_number(5.0));
    assert_eq!(env.get("fresh"), Some(exact_number(5.0)));
}

#[test]
fn set_requires_symbol_result_and_setq_requires_literal_symbol() {
    let env = Environment::root();
    env.define("x", exact_number(1.0));

    let set_err = run_exact(&env, 5, 0b00110, "42 9")
        .expect_err("SET target result must be symbol");
    assert!(format!("{set_err:?}").contains("Type"));

    let setq_err = run_exact(&env, 5, 0b00111, "(quote x) 9")
        .expect_err("SETQ target syntax must be literal symbol");
    assert!(format!("{setq_err:?}").contains("Type"));
    assert_eq!(env.get("x"), Some(exact_number(1.0)));
}
