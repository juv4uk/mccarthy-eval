//! Точна раціональна арифметика через самі функції СЕНС.
//!
//! `00001111` (ділення) — єдина з чотирьох арифметичних функцій, яка
//! породжує раціональні числа з цілих. Раніше її механізм потрапляв у
//! загальний шлях `+ - *` і панікував (`unreachable!`), хоча через ім'я
//! `/` ділення працювало.

use sens::{eval_program, ErrorKind, Session};

fn eval(source: &str) -> String {
    let mut session = Session::default();
    eval_program(source, &mut session)
        .unwrap_or_else(|error| panic!("{source}: {}", error.render(source)))
        .value
        .to_string()
}

#[test]
fn sens_division_produces_exact_rational_instead_of_panicking() {
    assert_eq!(eval("(00001111 1 3)"), "1/3");
    assert_eq!(eval("(00001111 6 3)"), "2");
    assert_eq!(eval("(00001111 1/3 1/6)"), "2");
}

#[test]
fn sens_arithmetic_closes_over_exact_rationals() {
    assert_eq!(eval("(00001100 1/3 1/6)"), "1/2");
    assert_eq!(eval("(00001101 1/10 1/10)"), "0");
    assert_eq!(eval("(00001110 1/3 3)"), "1");
    // (1/3 + 1/6) / (1/2) = 1 — жодного округлення на всьому шляху.
    assert_eq!(eval("(00001111 (00001100 1/3 1/6) 1/2)"), "1");
}

#[test]
fn sens_division_by_zero_fails_as_language_error_not_panic() {
    let mut session = Session::default();
    let error = eval_program("(00001111 1 0)", &mut session)
        .expect_err("division by exact zero must be a language error");
    assert_ne!(error.kind, ErrorKind::Parse);
}
