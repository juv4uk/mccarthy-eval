//! Static ownership guard only: migrated string/JSON names must not gain
//! duplicate dispatch inside the Rust evaluator. Their public laws live in Lisp.

#[test]
fn evaluator_source_does_not_dispatch_migrated_eager_names() {
    let evaluator = include_str!("../src/eval/mod.rs");
    for name in [
        "string-append",
        "string<?",
        "string?",
        "symbol->string",
        "string->symbol",
        "string-first",
        "string-rest",
        "codepoint->string",
        "string->codepoint",
        "sha256-hex",
        "json-parse",
    ] {
        assert!(
            !evaluator.contains(&format!("Some(\"{name}\")")),
            "evaluator regained hard-coded ownership of {name}"
        );
    }
}
