use sens::{eval_parsed_expressions, eval_program, load_core_library, load_process_library, parse_mixed_exact_domain_utf8_source, Session};

fn utf8_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();
    let source = include_str!("../../../lib/utf8.lisp");
    let exact = parse_mixed_exact_domain_utf8_source("lib/utf8.lisp", source)
        .expect("UTF-8 current exact-domain source must parse");
    eval_parsed_expressions(&exact, &mut session)
        .expect("UTF-8 source must load through its width-preserving reader");
    session
}

fn eval(source: &str) -> String {
    let mut session = utf8_session();
    eval_program(source, &mut session).unwrap().value.to_string()
}

#[test]
fn utf8_decodes_ascii_exactly() {
    assert_eq!(eval("(utf8-decode (quote (65 66 67)))"), "(decoded (65 66 67))");
}

#[test]
fn utf8_decodes_multibyte_scalars_exactly() {
    // U+00A2 CENT SIGN, U+20AC EURO SIGN, U+1F600 GRINNING FACE.
    assert_eq!(
        eval("(utf8-decode (quote (194 162 226 130 172 240 159 152 128)))"),
        "(decoded (162 8364 128512))"
    );
}

#[test]
fn utf8_decodes_valid_bytes_to_a_runtime_string_without_lossy_host_policy() {
    assert_eq!(
        eval("(utf8-decode-string (quote (65 194 162 226 130 172 240 159 152 128)))"),
        "(decoded \"A¢€😀\")"
    );
}

#[test]
fn utf8_encodes_runtime_strings_to_exact_wire_bytes() {
    assert_eq!(eval("(utf8-encode-string \"ABC\")"), "(65 66 67)");
    assert_eq!(
        eval("(utf8-encode-string \"A¢€😀\")"),
        "(65 194 162 226 130 172 240 159 152 128)"
    );
}

#[test]
fn utf8_encode_decode_round_trip_is_language_owned() {
    assert_eq!(
        eval("(utf8-decode-string (utf8-encode-string \"Привіт 😀\"))"),
        "(decoded \"Привіт 😀\")"
    );
}

#[test]
fn utf8_rejects_overlong_surrogate_and_out_of_range_sequences() {
    for bytes in [
        "(192 128)",       // overlong 2-byte form
        "(224 128 128)",   // overlong 3-byte form
        "(237 160 128)",   // UTF-16 surrogate U+D800
        "(244 144 128 128)", // above U+10FFFF
    ] {
        assert_eq!(
            eval(&format!("(utf8-decode (quote {bytes}))")),
            "(rejected invalid-utf8)"
        );
    }
}

#[test]
fn utf8_rejects_truncation_and_bad_continuation() {
    for bytes in ["(226 130)", "(240 159 152)", "(226 65 172)"] {
        assert_eq!(
            eval(&format!("(utf8-decode (quote {bytes}))")),
            "(rejected invalid-utf8)"
        );
    }
}

#[test]
fn utf8_rejects_non_bytes_before_protocol_interpretation() {
    assert_eq!(
        eval("(utf8-decode (quote (65 256 66)))"),
        "(rejected invalid-byte)"
    );
    assert_eq!(
        eval("(utf8-decode (quote (65 -1 66)))"),
        "(rejected invalid-byte)"
    );
}


fn explicitly_typed_d8_equal_call(d3: &[sens::Expr]) -> Vec<sens::Expr> {
    let sens::ExprKind::List(items) = &d3[0].kind else {
        panic!("потрібна форма-виклик")
    };
    let mut parts = items.to_vec();
    parts[0] = sens::Expr {
        kind: sens::ExprKind::DomainIdentity(sens::DomainIdentity::D8(
            sens::CoreD8::from_word(
                sens::Bit8::new(0b11110111).expect("ратифікована D8-координата"),
            ),
        )),
        span: parts[0].span,
    };
    vec![sens::Expr {
        kind: sens::ExprKind::List(std::rc::Rc::from(parts.into_boxed_slice())),
        span: d3[0].span,
    }]
}

#[test]
fn utf8_atomic_d3_eq_exhaustive_256_bytes_at_five_boundaries() {
    // Збіг восьми бітів із ратифікованим D8 не дає права auto-lift.
    let opaque = parse_mixed_exact_domain_utf8_source(
        "lib/utf8.lisp", "(11110111 0 0)",
    ).expect("історичні слова залишаються неявно невикличними");
    let sens::ExprKind::List(opaque_items) = &opaque[0].kind else {
        panic!("потрібна форма-виклик")
    };
    assert!(
        !matches!(&opaque_items[0].kind, sens::ExprKind::DomainIdentity(_)),
        "W8-переносник не надає чинної D8-тотожності"
    );

    // Контракт D3:101 EQ на атомах: однакова числова величина => D1:YES.
    // Вичерпні 256 байтів × 5 включених Unicode UTF8 меж = 1280 запусків.
    let mut session = Session::default();
    let mut checked = 0_usize;
    for byte in 0_u16..=255 {
        for boundary in [0_u16, 224, 237, 240, 244] {
            let source = format!("(101 {byte} {boundary})");
            let parsed = parse_mixed_exact_domain_utf8_source(
                "lib/utf8.lisp", &source,
            ).expect("точний виклик D3 EQ");
            let sens::ExprKind::List(items) = &parsed[0].kind else {
                panic!("потрібна форма-виклик")
            };
            assert!(matches!(
                &items[0].kind,
                sens::ExprKind::DomainIdentity(identity) if identity.width() == 3
            ), "{source}: ширина EQ мусить дорівнювати рівно трьом бітам");
            let result = eval_parsed_expressions(&parsed, &mut session)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            assert_eq!(
                result.value.as_predicate_bit(),
                Some(byte == boundary),
                "{source}: лише точний D1 результат, не Number і не Nil"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 256 * 5);
}

#[test]
fn utf8_d3_eq_rejects_nonatoms_and_d8_equal_remains_unadmitted() {
    let mut session = Session::default();
    let d3 = parse_mixed_exact_domain_utf8_source(
        "lib/utf8.lisp", "(101 (001 (a b)) (001 (a b)))",
    ).expect("синтаксис допустимий");
    let error = eval_parsed_expressions(&d3, &mut session)
        .expect_err("D3 EQ є функцією лише атомарної рівності");
    assert_eq!(error.kind, sens::ErrorKind::Type);

    // D8:EQUAL має окрему ратифіковану тотожність, але сьогодні
    // її виклик у цьому середовищі НЕ допущений. Не відкривати його тестом.
    let d8 = explicitly_typed_d8_equal_call(&d3);
    let sens::ExprKind::List(parts) = &d8[0].kind else {
        panic!("потрібна форма-виклик")
    };
    assert!(matches!(
        &parts[0].kind,
        sens::ExprKind::DomainIdentity(identity) if identity.width() == 8
    ));
    let blocked = eval_parsed_expressions(&d8, &mut session)
        .expect_err("існування D8:11110111 не є дозволом виконання");
    assert_eq!(blocked.kind, sens::ErrorKind::Type);
    assert!(blocked.message.contains("D8 11110111"), "{}", blocked.message);
}

#[test]
fn utf8_layer_keeps_process_library_as_a_separate_top_level_load() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    load_process_library(&mut session)
        .expect("UTF-8, process and TCP language layers must load sequentially");
}

#[test]
fn utf8_helper_returns_exact_d1_predicate_on_boundaries() {
    let mut session = utf8_session();
    for (source, expected) in [
        ("(utf8-in-range? 0 0 255)", true),
        ("(utf8-in-range? 255 0 255)", true),
        ("(utf8-in-range? 256 0 255)", false),
        ("(utf8-in-range? -1 0 255)", false),
        ("(utf8-continuation-byte? 127)", false),
        ("(utf8-continuation-byte? 128)", true),
        ("(utf8-continuation-byte? 191)", true),
        ("(utf8-continuation-byte? 192)", false),
        ("(utf8-byte? 0)", true),
        ("(utf8-byte? 255)", true),
        ("(utf8-byte? 256)", false),
        ("(utf8-all-bytes? (quote ()))", true),
        ("(utf8-all-bytes? (quote (0 127 255)))", true),
        ("(utf8-all-bytes? (quote (0 256)))", false),
        ("(utf8-two-continuations? 128 191)", true),
        ("(utf8-two-continuations? 128 192)", false),
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("{source} failed: {e:?}")).value;
        assert_eq!(
            actual.as_predicate_bit(), Some(expected),
            "{source}: expected an exact D1 bit, never ordinary Number 0/1"
        );
    }
}

const UTF8_D1_LAW: &str =
    include_str!("../../../tests/semantic/utf8-direct-d1-law.lisp");

/// The executable SENS program compares actual/expected D1 itself.
/// Rust only transports the fixture, executes it, and observes the final bit.
/// No Python or Rust function duplicates the UTF-8 predicate oracle.
#[test]
fn utf8_direct_d1_self_verdict_and_one_bit_mutant() {
    let program = sens::parse_mixed_exact_domain_utf8_source(
        "lib/utf8.lisp", UTF8_D1_LAW,
    ).expect("the current pinned UTF-8 reader must parse binary SENS law");
    assert_eq!(program.len(), 2, "two structural SENS-owned UTF-8 verdicts; numeric D24 boundaries remain transitional");

    let mut session = utf8_session();
    for (index, form) in program.iter().enumerate() {
        let result = sens::eval_parsed_expressions(std::slice::from_ref(form), &mut session)
            .unwrap_or_else(|error| {
                panic!("test_id=utf8-d1-direct layer=eval fixture=tests/semantic/utf8-direct-d1-law.lisp form={index} error={error}")
            });
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(true),
            "test_id=utf8-d1-direct form={index} SENS self-verdict was not D1:YES: {}",
            result.value
        );
    }

    let mutant_source = UTF8_D1_LAW.replacen("#d1:0", "#d1:1", 1);
    assert_eq!(mutant_source.len(), UTF8_D1_LAW.len(), "one bit, no frame drift");
    let mutations = UTF8_D1_LAW.bytes().zip(mutant_source.bytes())
        .filter(|(before, after)| before != after).count();
    assert_eq!(mutations, 1, "the mutant must change only one D1 bit");
    let mutant = sens::parse_mixed_exact_domain_utf8_source(
        "lib/utf8.lisp", &mutant_source,
    ).expect("one-bit mutant remains valid SENS source");
    let mut mutant_session = utf8_session();
    assert!(
        sens::eval_parsed_expressions(std::slice::from_ref(&mutant[0]), &mut mutant_session).is_err(),
        "SENS itself must fail closed on the one-bit false verdict"
    );
}
