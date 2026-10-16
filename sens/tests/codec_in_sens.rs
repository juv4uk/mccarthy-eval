//! Компактний кодек, написаний SENS-ом, у ТОЧНОМУ D2-порядку T5 (lib/codec-d2.lisp).
//! Маркери: a=open, c=sep, b=close, d=dot (1:1 до слів 10/00/01/11).
use sens::{eval_parsed_expressions, eval_program, load_core_library, parse_mixed_exact_domain, Session};

fn codec_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("current Core library");
    let forms = parse_mixed_exact_domain(include_str!("../../../lib/codec-d2.lisp"))
        .expect("codec-d2 current D3/D4 source must parse");
    eval_parsed_expressions(&forms, &mut session).expect("codec-d2 source must load");
    session
}
fn eval(source: &str, session: &mut Session) -> String {
    eval_program(source, session).unwrap_or_else(|e| panic!("{source} failed: {e}")).value.to_string()
}

#[test]
fn codec_emits_exact_t5_d2_order() {
    let mut s = codec_session();
    // README: (CAR (CONS () ())) -> 10 100 00 10 111 00 000 00 000 01 01
    assert_eq!(
        eval("(t5-encode (quote (100 (111 () ()))))", &mut s),
        eval("(quote (d2-open 100 d2-sep d2-open 111 d2-sep () d2-sep () d2-close d2-close))", &mut s)
    );
}

// Exact structural D2 order, tested through compact SENS-owned marker proxies.
// This is not yet proof of physical two-bit tokens in a packed .sens file.
#[test]
fn codec_distinguishes_d2_dot_from_separator() {
    let mut s = codec_session();
    for (source, expected) in [
        ("100", "(100)"),
        ("(100 111)", "(d2-open 100 d2-sep 111 d2-close)"),
        ("(100 . 111)", "(d2-open 100 d2-dot 111 d2-close)"),
    ] {
        let encoded = eval(&format!("(t5-encode (quote {source}))"), &mut s);
        let expected_stream = eval(&format!("(quote {expected})"), &mut s);
        assert_eq!(
            encoded, expected_stream,
            "D2 marker order mismatch: source={source} expected={expected}"
        );
        let returned = eval(&format!("(t5-roundtrip (quote {source}))"), &mut s);
        let wanted = eval(&format!("(quote {source})"), &mut s);
        assert_eq!(returned, wanted, "D2 roundtrip mismatch: source={source}");
    }
}

#[test]
fn codec_round_trips_every_shape() {
    let mut s = codec_session();
    for src in ["101", "()", "(100 111 000)", "(100 . 111)", "((100) 111)"] {
        let got = eval(&format!("(t5-roundtrip (quote {src}))"), &mut s);
        let want = eval(&format!("(quote {src})"), &mut s);
        assert_eq!(got, want, "round-trip for {src}");
    }
}
