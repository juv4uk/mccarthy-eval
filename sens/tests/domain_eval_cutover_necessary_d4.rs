//! Зовнішній позитивний/негативний свідок: звичайні імена Lisp
//! повинні проходити точно через ратифіковану D4, не через SID8.
use sens::{eval_expr, parse, Environment, Value};

fn first_form(source: &str) -> sens::Expr {
    parse(source).expect("коректний Lisp-source")
        .into_iter().next().expect("одна форма")
}

#[test]
fn ukrainian_lambda_runs_on_unlowered_ast() {
    let root = Environment::root();
    let form = first_form("(функція (x) x)");
    let closure = eval_expr(&form, &root)
        .expect("D4:0010 LAMBDA за українською проєкцією");
    assert!(matches!(closure, Value::Closure(_)));
}

#[test]
fn ukrainian_define_binds_in_current_environment() {
    let root = Environment::root();
    let form = first_form("(визначити перевірка ())");
    eval_expr(&form, &root)
        .expect("D4:0011 DEFINE за українською проєкцією");
    assert!(matches!(root.get("перевірка"), Some(Value::Nil)));
}

#[test]
fn ordinary_user_function_does_not_inherit_d4_mechanism() {
    let root = Environment::root();
    let form = first_form("(чужа-функція x)");
    let error = eval_expr(&form, &root)
        .expect_err("невідома функція не може отримати механізм D4");
    assert!(matches!(error.kind, sens::ErrorKind::UnknownSymbol | sens::ErrorKind::Type));
}
