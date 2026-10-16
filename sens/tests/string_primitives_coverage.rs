use sens::{eval_program, Session};

fn eval_src(src: &str) -> String {
    let mut session = Session::default();
    let result = eval_program(src, &mut session)
        .unwrap_or_else(|e| panic!("eval should succeed for {src:?}: {e}"));
    // Value::String Display wraps in quotes; unwrap them
    let s = result.value.to_string();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s
    }
}

#[test]
fn string_slice_cyrillic() {
    assert_eq!(eval_src(r#"(string-slice "привіт світ" 0 6)"#), "привіт");
}

#[test]
fn string_slice_clamps_at_end() {
    assert_eq!(eval_src(r#"(string-slice "abc" 2 99)"#), "c");
}

#[test]
fn unicode_char_boundaries_respected() {
    assert_eq!(
        eval_src(r#"(string-slice "Привіт Володимир" 0 7)"#),
        "Привіт "
    );
}

#[test]
fn empty_string_slice_returns_empty() {
    assert_eq!(eval_src(r#"(string-slice "" 0 5)"#), "");
}

#[test]
fn string_slice_single_char() {
    assert_eq!(eval_src(r#"(string-slice "x" 0 1)"#), "x");
}

#[test]
fn sanskrit_iast_text_slicing() {
    assert_eq!(eval_src(r#"(string-slice "dharma" 0 3)"#), "dha");
}

#[test]
fn full_range_returns_full_string() {
    assert_eq!(eval_src(r#"(string-slice "тест" 0 4)"#), "тест");
}
