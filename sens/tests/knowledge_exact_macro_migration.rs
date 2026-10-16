use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source, eval_program,
    parse_mixed_exact_domain_reasoning_source, Session,
};

const KNOWLEDGE: &str = include_str!("../../../lib/knowledge.lisp");

fn load_reasoning(session: &mut Session, path: &str, source: &str) {
    let parsed = parse_mixed_exact_domain_reasoning_source(path, source)
        .unwrap_or_else(|e| panic!("pinned reasoning parse failed for {path}: {e}"));
    eval_parsed_expressions(&parsed, session)
        .unwrap_or_else(|e| panic!("pinned reasoning load failed for {path}: {e}"));
}

fn session() -> Session {
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("exact-domain core bootstrap");

    for (path, source) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", KNOWLEDGE),
    ] {
        load_reasoning(&mut session, path, source);
    }
    session
}

fn eval(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

fn eight_bit_heads(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_string = true;
            i += 1;
            continue;
        }

        if b == b'(' && i + 9 < bytes.len() {
            let bits = &bytes[i + 1..i + 9];
            let delimiter = bytes[i + 9];
            if bits.iter().all(|b| matches!(b, b'0' | b'1'))
                && (delimiter.is_ascii_whitespace() || delimiter == b')')
            {
                result.push(String::from_utf8(bits.to_vec()).expect("binary head is ASCII"));
            }
        }
        i += 1;
    }
    result
}

#[test]
fn knowledge_file_has_no_legacy_defmacro_head_and_preserves_real_d8_width() {
    assert!(
        !KNOWLEDGE.contains("(00001010 "),
        "DEFMACRO compatibility head must not survive in migrated knowledge.lisp"
    );

    let mut heads = eight_bit_heads(KNOWLEDGE);
    heads.sort();
    heads.dedup();

    assert_eq!(
        heads,
        vec![
            "11101010".to_string(),
            "11101101".to_string(),
            "11110100".to_string(),
        ],
        "surviving 8-bit executable heads must be genuine D8 identities"
    );
}

#[test]
fn migrated_defmodule_macro_drives_backward_and_forward_reasoning() {
    let mut session = session();

    eval(
        &mut session,
        r#"
        (defmodule archive
          (quote (
            ((planet mars))
            ((valuable (var x)) (planet (var x)))
          )))
        "#,
    );

    assert_eq!(
        eval(
            &mut session,
            "(reason-in (quote archive) (quote (planet mars)))",
        ),
        "((() (proved (planet mars) (planet mars) ())))"
    );

    assert_eq!(
        eval(&mut session, "(forward-in (quote archive))"),
        "((valuable mars) (planet mars))"
    );
}
