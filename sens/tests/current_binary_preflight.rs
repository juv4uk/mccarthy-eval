//! Canonical SENS preflight: the authority of a source word is its exact
//! binary width and payload, never a human spelling or historical SID8.
//! Ratified language laws remain in lib/domains/*.lisp and their oracles.

use sens::{eval_parsed_expressions, load_core_library, parse_canonical_binary, wire_decode_program, wire_encode_program, Expr, ExprKind, Session, Value};

fn binary_trace(expression: &Expr) -> String {
    match &expression.kind {
        ExprKind::DomainIdentity(identity) => {
            format!("D{}:{:0width$b}", identity.width(), identity.packed_bits(), width = identity.width())
        }
        ExprKind::List(items) => {
            let children = items.iter().map(binary_trace).collect::<Vec<_>>();
            format!("({})", children.join(" "))
        }
        ExprKind::Pair(head, tail) => format!("({} . {})", binary_trace(head), binary_trace(tail)),
        // Canonical source and its physical wire may never invent surface names,
        // flat SIDs, or implicit callable admission from a naked binary width.
        other => panic!("non-binary source identity entered canonical carrier: {other:?}"),
    }
}

fn only(source: &str) -> Expr {
    let expressions = parse_canonical_binary(source).expect("exact binary words parse");
    assert_eq!(expressions.len(), 1);
    expressions.into_iter().next().unwrap()
}

#[test]
fn each_current_binary_width_preserves_its_own_coordinate() {
    // A width, not a numeric payload alone, identifies the rung.
    // D2 is structural framing, not a standalone value word.
    for width in [1usize, 3, 4, 5, 6, 7, 8, 9] {
        for bits in [1u16, (1u16 << width) - 1] {
            let token = format!("{bits:0width$b}");
            let expression = only(&token);
            let ExprKind::DomainIdentity(identity) = expression.kind else {
                panic!("D{width} word changed from exact coordinate to surface");
            };
            assert_eq!(identity.width(), width);
            assert_eq!(identity.packed_bits(), bits);
            assert_eq!(identity.to_string(), token);
        }
    }

    // D3 000 alone is the exact structural empty value, never a function ID.
    assert!(matches!(only("000").kind, ExprKind::List(ref xs) if xs.is_empty()));
    assert_ne!(binary_trace(&only("1")), binary_trace(&only("000000001")));
}

#[test]
fn every_nonempty_d3_binary_head_round_trips_without_english_or_sid() {
    // Check the seven nonempty 3-bit coordinates mechanically. The Lisp
    // domain table, not this Rust test, determines each operation's meaning.
    for bits in 1u8..=7 {
        let head = format!("{bits:03b}");
        let source = format!("10 {head} 00 000 01");
        let expressions = parse_canonical_binary(&source).expect("D2-framed D3 word");
        let ExprKind::List(items) = &expressions[0].kind else {
            panic!("D2 structure must not become a named operation");
        };
        assert_eq!(items.len(), 2);
        let ExprKind::DomainIdentity(identity) = items[0].kind else {
            panic!("D3 head was routed through legacy SID");
        };
        assert_eq!((identity.width(), identity.packed_bits()), (3, u16::from(bits)));
        let expected = binary_trace(&expressions[0]);
        let recovered = wire_decode_program(&wire_encode_program(&expressions))
            .expect("width-preserving program wire");
        assert_eq!(recovered.len(), 1);
        assert_eq!(binary_trace(&recovered[0]), expected);
    }
}

#[test]
fn d7_payloads_remain_structural_data_without_implicit_text_names() {
    let expression = only("10 1000001 00 1000010 01");
    let ExprKind::List(items) = expression.kind else {
        panic!("a D2 list of W7 coordinates must remain a list");
    };
    assert_eq!(items.len(), 2);
    for (item, bits) in items.iter().zip([65u16, 66]) {
        let ExprKind::DomainIdentity(identity) = item.kind else {
            panic!("a raw W7 cell must not become an implicit Text7 symbol");
        };
        assert_eq!((identity.width(), identity.packed_bits()), (7, bits));
        assert!(identity.core_operation().is_none());
    }
}

#[test]
fn mixed_width_binary_words_do_not_collapse_to_eight_bit_ids() {
    let source = "10 1 00 001 00 0001 00 00001 00 000001 00 0000001 00 00000001 00 000000001 01";
    let expression = only(source);
    let expected = binary_trace(&expression);
    let ExprKind::List(items) = &expression.kind else {
        panic!("width-qualified words must remain D2 structural children");
    };
    let widths = items.iter().map(|item| match item.kind {
        ExprKind::DomainIdentity(identity) => identity.width(),
        ref other => panic!("binary word silently coerced to flat SID: {other:?}"),
    }).collect::<Vec<_>>();
    assert_eq!(widths, [1, 3, 4, 5, 6, 7, 8, 9]);
    let decoded = wire_decode_program(&wire_encode_program(&[expression]))
        .expect("mixed width wire");
    assert_eq!(binary_trace(&decoded[0]), expected);
}

#[test]
fn human_names_and_nonbinary_tokens_never_enter_canonical_source() {
    for source in ["(001)", "car", "CONS", "010xyz", "2", "10 001"] {
        assert!(parse_canonical_binary(source).is_err(), "{source}");
    }
}

#[test]
fn current_cond_reference_is_valid_binary_and_executes_without_legacy_sid() {
    let source = include_str!("../../../examples/binary-language/current-cond-reference.lisp");
    let words = source.split_whitespace().collect::<Vec<_>>();
    assert!(!words.is_empty());
    assert!(
        words.iter().all(|word| word.bytes().all(|byte| matches!(byte, b'0' | b'1'))),
        "the reference file must contain only visible binary words and whitespace"
    );
    assert!(
        words.iter().all(|word| (1..=4).contains(&word.len())),
        "the current COND reference must not contain legacy W8/W9 identities"
    );

    let expressions = parse_canonical_binary(source).expect("current D3 COND binary source");
    assert_eq!(expressions.len(), 1);
    let result = eval_parsed_expressions(&expressions, &mut Session::default())
        .expect("two-field COND must execute with exact D1 predicate results");
    assert!(matches!(result.value, Value::Nil));
    assert!(result.output.is_empty());
}


/// Regression for obsolete numeric local-binding heads in mirrored Core files.
/// The lexical `let` macro is Lisp-owned; 10011100 is not its current identity.
#[test]
fn core4_local_binding_heads_are_current_and_postcore_libraries_load() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("Core4 bootstrap");

    for (library, source) in [
        ("time", include_str!("../../../lib/time.lisp")),
        ("process", include_str!("../../../lib/process.lisp")),
    ] {
        // Обидва файли мають власний точнодоменний reader; звичайний
        // Lisp-парсер хибно перетворює короткі двійкові голови на числа.
        let forms = if library == "time" {
            sens::parse_mixed_exact_domain_time_source("lib/time.lisp", source)
        } else {
            sens::parse_mixed_exact_domain_process_tcp_source("lib/process.lisp", source)
        }
        .unwrap_or_else(|error| panic!("{library} parse: {error}"));
        for (index, form) in forms.iter().enumerate() {
            eval_parsed_expressions(std::slice::from_ref(form), &mut session)
                .unwrap_or_else(|error| panic!(
                    "{library} form {} byte {} failed: {}",
                    index + 1, form.span.start, error
                ));
        }
    }
}

#[test]
fn time_library_reader_rejects_foreign_path_instead_of_inferring_domain_from_bits() {
    let source = include_str!("../../../lib/time.lisp");
    assert!(sens::parse_mixed_exact_domain_time_source("lib/process.lisp", source).is_err());
    assert!(sens::parse_mixed_exact_domain_time_source("lib/not-time.lisp", source).is_err());
}

#[test]
fn process_library_reader_is_path_bound_and_preserves_quoted_data() {
    let source = "(0011 probe (0010 (x) (001 (101 x))))";
    assert!(sens::parse_mixed_exact_domain_process_tcp_source("lib/time.lisp", source).is_err());
    assert!(sens::parse_mixed_exact_domain_process_tcp_source("lib/not-process.lisp", source).is_err());

    let parsed = sens::parse_mixed_exact_domain_process_tcp_source("lib/process.lisp", source)
        .expect("owner-bound process source");
    assert_eq!(parsed.len(), 1);
    let ExprKind::List(definition) = &parsed[0].kind else { panic!("definition"); };
    let ExprKind::List(lambda) = &definition[2].kind else { panic!("lambda"); };
    let ExprKind::List(quote_form) = &lambda[2].kind else { panic!("quoted body"); };
    assert!(matches!(&quote_form[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b001),
        "D3 QUOTE remains an exact identity");
    let ExprKind::List(payload) = &quote_form[1].kind else { panic!("quoted payload"); };
    assert!(
        !matches!(&payload[0].kind, ExprKind::DomainIdentity(_)),
        "numeric-looking payload under QUOTE must not become an executable identity"
    );
}

/// Diagnose the first non-callable top-level time form after Core4 bootstrap.
/// This test intentionally preserves strict runtime behavior: it reports the
/// exact top-level index and source bytes instead of replacing the failed form.
#[test]
fn time_library_top_level_replay_reports_first_noncallable_form() {
    let mut session = Session::default();
    sens::load_core_library(&mut session).expect("Core4 bootstrap");
    let source = include_str!("../../../lib/time.lisp");
    let parsed = sens::parse_mixed_exact_domain_time_source("lib/time.lisp", source)
        .expect("time library through its admitted exact-domain reader");

    for (index, expression) in parsed.iter().enumerate() {
        if let Err(error) =
            eval_parsed_expressions(std::slice::from_ref(expression), &mut session)
        {
            let start = (expression.span.start as usize).min(source.len());
            let end = (expression.span.end as usize).min(source.len()).max(start);
            let preview_end = end.min(start.saturating_add(180));
            let snippet = String::from_utf8_lossy(&source.as_bytes()[start..preview_end]);
            panic!(
                "TIME-DIAGNOSTIC: top_level_index={} source_span={}..{} source={:?} error={:?}",
                index + 1,
                start,
                end,
                snippet.replace('\n', " "),
                error
            );
        }
    }
}


/// Compare the actual whole-file loader route with the successful formwise
/// replay above. This diagnostic prints the first enclosing top-level form
/// if batch evaluation fails, but does not turn the finding into fake success
/// or suppress any runtime error in the loader itself.
#[test]
fn time_library_loader_reports_first_failing_form() {
    let source = include_str!("../../../lib/time.lisp");
    let parsed = sens::parse_mixed_exact_domain_time_source("lib/time.lisp", source)
        .expect("time library through its admitted exact-domain reader");
    let mut session = Session::default();
    sens::load_core_library(&mut session).expect("Core4 bootstrap");

    sens::load_time_library(&mut session).unwrap_or_else(|error| {
        let start = (error.span.start as usize).min(source.len());
        let end = (error.span.end as usize).min(source.len()).max(start);
        let form = parsed.iter().enumerate().find(|(_, expression)| {
            let form_start = expression.span.start as usize;
            let form_end = expression.span.end as usize;
            form_start <= start && end <= form_end
        });
        let (form_label, snippet) = form
            .map(|(index, expression)| {
                let form_start = (expression.span.start as usize).min(source.len());
                let form_end = (expression.span.end as usize).min(source.len()).max(form_start);
                let preview_end = form_end.min(form_start.saturating_add(180));
                (
                    format!("{}:{}..{}", index + 1, expression.span.start, expression.span.end),
                    String::from_utf8_lossy(&source.as_bytes()[form_start..preview_end]).replace('\n', " "),
                )
            })
            .unwrap_or_else(|| ("unmapped".to_owned(), "<no containing top-level form>".to_owned()));
        panic!(
            "TIME-LOADER-REGRESSION: error_span={}..{} enclosing_form={} source={:?} error={:?}",
            start, end, form_label, snippet, error
        );
    });
}
