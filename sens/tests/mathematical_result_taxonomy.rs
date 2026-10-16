//! #225 observer for the Lisp-owned mathematical-result taxonomy.
//! Rust transports taxonomy bytes and runtime actuals only.

use std::fs;
use std::path::PathBuf;

use sens::{eval_program, load_core_library, parse, Expr, ExprKind, Session};

#[derive(Clone)]
struct Row {
    source: String,
    expr: String,
}

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn alist_str<'a>(entries: &'a [Expr], key: &str) -> Option<&'a str> {
    entries.iter().find_map(|entry| {
        let ExprKind::Pair(k, v) = &entry.kind else {
            return None;
        };
        let ExprKind::Symbol(name) = &k.kind else {
            return None;
        };
        if &**name != key {
            return None;
        }
        match &v.kind {
            ExprKind::String(value) => Some(value.as_ref()),
            _ => None,
        }
    })
}

fn alist_true(entries: &[Expr], key: &str) -> bool {
    entries.iter().any(|entry| {
        let ExprKind::Pair(k, v) = &entry.kind else {
            return false;
        };
        let ExprKind::Symbol(name) = &k.kind else {
            return false;
        };
        &**name == key && matches!(&v.kind, ExprKind::Symbol(value) if &**value == "t")
    })
}

fn rows() -> Vec<Row> {
    let source = include_str!("../../../tests/fixtures/mathematical-result-v1.lisp");
    parse(source)
        .expect("mathematical-result-v1.lisp must parse")
        .into_iter()
        .filter_map(|form| {
            let ExprKind::List(entries) = &form.kind else {
                return None;
            };
            if !alist_true(entries, "active") {
                return None;
            }
            Some(Row {
                source: source[form.span.start..form.span.end].to_string(),
                expr: alist_str(entries, "expr")?.to_string(),
            })
        })
        .collect()
}

fn escape_lisp_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn load_witness_runner(session: &mut Session) {
    let source = fs::read_to_string(repo_file("tests/fixtures/witness-runner.lisp"))
        .expect("Lisp-owned witness runner");
    eval_program(&source, session).expect("witness-runner.lisp must load");
}

fn transport_taxonomy(session: &mut Session) {
    let source = fs::read_to_string(repo_file("contracts/mathematical-result-taxonomy.lisp"))
        .expect("#225 requires Lisp-owned contracts/mathematical-result-taxonomy.lisp");
    let forms = parse(&source).expect("mathematical-result-taxonomy.lisp must parse");
    assert_eq!(
        forms.len(),
        1,
        "#225 taxonomy must remain one self-contained Lisp data document"
    );
    let form = &forms[0];
    let exact = &source[form.span.start..form.span.end];
    eval_program(
        &format!("(def mathematical-result-taxonomy-document (quote {exact}))"),
        session,
    )
    .expect("transport mathematical-result taxonomy");
}

#[test]
fn existing_arithmetic_results_remain_mathematical_values() {
    let rows = rows();
    assert_eq!(rows.len(), 4, "#225 current-result anchor must retain four rows");

    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    load_witness_runner(&mut session);

    for row in &rows {
        let result = eval_program(&row.expr, &mut session)
            .unwrap_or_else(|error| panic!("mathematical result {} failed: {error}", row.expr));
        let actual = format!("(value \"{}\")", escape_lisp_string(&result.value.to_string()));
        let program = format!(
            "(witness-pass? (witness-verdict (quote {}) (quote {})))",
            row.source, actual
        );
        let verdict = eval_program(&program, &mut session)
            .unwrap_or_else(|error| panic!("Lisp verdict failed for {}: {error}", row.expr))
            .value
            .to_string();
        assert_eq!(verdict, "t", "#225 Lisp witness rejected {} as a mathematical value", row.expr);
    }
}

#[test]
fn lisp_owned_mathematical_result_taxonomy_is_self_consistent() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    transport_taxonomy(&mut session);
    let witness = fs::read_to_string(repo_file(
        "tests/fixtures/mathematical-result-taxonomy-witness.lisp",
    ))
    .expect("#225 mathematical taxonomy witness");
    eval_program(&witness, &mut session).expect("mathematical taxonomy witness must load");
    let verdict = eval_program("(mathematical-result-taxonomy-witness)", &mut session)
        .expect("mathematical taxonomy witness must execute")
        .value
        .to_string();
    assert!(
        verdict.starts_with("(mathematical-result-taxonomy-witness (status pass)"),
        "Lisp-owned mathematical taxonomy rejected itself: {verdict}"
    );
}
