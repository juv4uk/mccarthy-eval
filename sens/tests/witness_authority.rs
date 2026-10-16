//! WITNESS-CORPUS-1 (#113): the corpus owns expected truth; Rust only
//! transports actual outcomes and asks SENS-owned witness logic for a verdict.

use std::fs;
use std::path::PathBuf;

use sens::{
    eval_program, load_core_library, load_meta_evaluator_library, parse, Expr, ExprKind, Session,
    Sens8,
};

#[derive(Clone)]
struct WitnessRow {
    source: String,
    expr: String,
    expected: Option<String>,
    error: Option<String>,
    meta_eval: bool,
    compiler_corpus: bool,
}

#[derive(Clone)]
struct D3PredicateBoundaryRow {
    case: String,
    expr: String,
    expected_bit: bool,
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

fn alist_sid(entries: &[Expr], key: &str) -> Option<Sens8> {
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
            ExprKind::Sid(sid) => Some(*sid),
            other => panic!("{key} must be exact bare Sens8, got {other:?}"),
        }
    })
}

fn alist_flag(entries: &[Expr], key: &str) -> bool {
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

fn transition_rows() -> Vec<(String, WitnessRow)> {
    let source = include_str!("../../../tests/fixtures/conformance-transition-witness.lisp");
    parse(source)
        .expect("conformance-transition-witness.lisp must parse")
        .into_iter()
        .filter_map(|form| {
            let ExprKind::List(entries) = &form.kind else {
                return None;
            };
            Some((
                alist_str(entries, "supersedes-expr")?.to_string(),
                WitnessRow {
                    source: source[form.span.start..form.span.end].to_string(),
                    expr: alist_str(entries, "expr")?.to_string(),
                    expected: alist_str(entries, "expected").map(str::to_string),
                    error: alist_str(entries, "error").map(str::to_string),
                    meta_eval: alist_flag(entries, "meta-eval"),
                    compiler_corpus: alist_flag(entries, "compiler-corpus"),
                },
            ))
        })
        .collect()
}

fn witness_rows() -> Vec<WitnessRow> {
    let transitions = transition_rows();
    let source = include_str!("../../../tests/fixtures/conformance.lisp");
    let mut rows: Vec<_> = parse(source)
        .expect("conformance.lisp must parse")
        .into_iter()
        .filter_map(|form| {
            let ExprKind::List(entries) = &form.kind else {
                return None;
            };
            let compiler_corpus = alist_flag(entries, "compiler-corpus");
            let meta_eval = alist_flag(entries, "meta-eval");
            if !compiler_corpus && !meta_eval {
                return None;
            }
            let expr = alist_str(entries, "expr")?.to_string();
            if transitions
                .iter()
                .any(|(supersedes_expr, _)| supersedes_expr == &expr)
            {
                return None;
            }
            Some(WitnessRow {
                source: source[form.span.start..form.span.end].to_string(),
                expr,
                expected: alist_str(entries, "expected").map(str::to_string),
                error: alist_str(entries, "error").map(str::to_string),
                meta_eval,
                compiler_corpus,
            })
        })
        .collect();

    rows.extend(transitions.into_iter().map(|(_, row)| row));
    rows
}

fn d3_predicate_boundary_rows() -> Vec<D3PredicateBoundaryRow> {
    let source = include_str!("../../../tests/fixtures/d3-predicate-boundary-cases.lisp");
    let forms = parse(source).expect("D3 predicate boundary corpus must parse as Lisp data");
    assert_eq!(forms.len(), 1, "D3 predicate boundary corpus must be one data form");

    let ExprKind::List(rows) = &forms[0].kind else {
        panic!("D3 predicate boundary corpus root must be a list");
    };

    rows.iter()
        .map(|row| {
            let ExprKind::List(entries) = &row.kind else {
                panic!("each D3 predicate boundary row must be an alist");
            };
            let case = alist_str(entries, "випадок")
                .expect("predicate boundary row needs випадок")
                .to_string();
            let expr = alist_str(entries, "вираз")
                .expect("predicate boundary row needs вираз")
                .to_string();
            let expected_bit = match alist_str(entries, "очікуваний-біт")
                .expect("predicate boundary row needs очікуваний-біт")
            {
                "1" => true,
                "0" => false,
                other => panic!("predicate boundary bit must be 0/1, got {other:?}"),
            };
            D3PredicateBoundaryRow {
                case,
                expr,
                expected_bit,
            }
        })
        .collect()
}

fn canon_zero_rows() -> Vec<WitnessRow> {
    let source = include_str!("../../../tests/fixtures/canon-zero-v2.lisp");
    parse(source)
        .expect("canon-zero-v2.lisp must parse")
        .into_iter()
        .filter_map(|form| {
            let ExprKind::List(entries) = &form.kind else {
                return None;
            };
            // #215 is deliberately two-phase. The fixture keeps already-proven
            // future RED targets, but only rows explicitly marked active are
            // executable before #218/#217 replace structural T/NIL control.
            if !alist_flag(entries, "active") {
                return None;
            }
            Some(WitnessRow {
                source: source[form.span.start..form.span.end].to_string(),
                expr: alist_str(entries, "expr")?.to_string(),
                expected: alist_str(entries, "expected").map(str::to_string),
                error: alist_str(entries, "error").map(str::to_string),
                meta_eval: false,
                compiler_corpus: false,
            })
        })
        .collect()
}

fn structure_core_rows() -> Vec<WitnessRow> {
    let source = include_str!("../../../tests/fixtures/structure-core-v1.lisp");
    parse(source)
        .expect("structure-core-v1.lisp must parse")
        .into_iter()
        .filter_map(|form| {
            let ExprKind::List(entries) = &form.kind else {
                return None;
            };
            if !alist_flag(entries, "structure-core") {
                return None;
            }
            alist_sid(entries, "semantic-id")
                .expect("every structure-core row must carry exact bare Sens8 identity");
            Some(WitnessRow {
                source: source[form.span.start..form.span.end].to_string(),
                expr: alist_str(entries, "expr")?.to_string(),
                expected: alist_str(entries, "expected").map(str::to_string),
                error: alist_str(entries, "error").map(str::to_string),
                meta_eval: true,
                compiler_corpus: false,
            })
        })
        .collect()
}

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn load_witness_library(session: &mut Session) {
    let source = fs::read_to_string(repo_file("tests/fixtures/witness-runner.lisp"))
        .expect("#113 requires Lisp-owned witness runner/comparator fixture");
    eval_program(&source, session).expect("witness-runner.lisp must load");
}

fn transport_island_compat_document(session: &mut Session) {
    let source = fs::read_to_string(repo_file("contracts/island-compat-contract.lisp"))
        .expect("#749 requires the Lisp-owned island compatibility contract");
    let forms = parse(&source).expect("island-compat-contract.lisp must be readable Lisp data");
    assert_eq!(
        forms.len(),
        1,
        "#749 island compatibility contract must remain one self-contained Lisp data document"
    );

    let form = &forms[0];
    let exact_form_source = &source[form.span.start..form.span.end];
    let transport = format!("(def island-compat-document (quote {exact_form_source}))");
    eval_program(&transport, session)
        .expect("host observer must be able to transport island contract bytes into Lisp data");
}

fn load_island_compat_witness(session: &mut Session) {
    let source = fs::read_to_string(repo_file("tests/fixtures/island-compat-witness.lisp"))
        .expect("#749 requires its Lisp-owned island compatibility witness");
    eval_program(&source, session).expect("island-compat-witness.lisp must load");
}

fn transport_life_1_document(session: &mut Session) {
    let source = fs::read_to_string(repo_file("contracts/life-1-contract.lisp"))
        .expect("#785 requires the Lisp-owned LIFE-1 contract");
    let forms = parse(&source).expect("life-1-contract.lisp must be readable Lisp data");
    assert_eq!(
        forms.len(),
        1,
        "#785 LIFE-1 contract must remain one self-contained Lisp data document"
    );

    let form = &forms[0];
    let exact_form_source = &source[form.span.start..form.span.end];
    let transport = format!("(def life-1-document (quote {exact_form_source}))");
    eval_program(&transport, session)
        .expect("host observer must transport LIFE-1 contract bytes into Lisp data");
}

fn load_life_1_contract_witness(session: &mut Session) {
    let source = fs::read_to_string(repo_file("tests/fixtures/life-1-contract-witness.lisp"))
        .expect("#785 requires its Lisp-owned LIFE-1 witness");
    eval_program(&source, session).expect("life-1-contract-witness.lisp must load");
}

fn transport_life_substrate_parity_document(session: &mut Session) {
    let source = fs::read_to_string(repo_file("contracts/life-substrate-parity-809.lisp"))
        .expect("#809 requires the Lisp-owned substrate parity contract");
    let forms = parse(&source).expect("life-substrate-parity-809.lisp must be readable Lisp data");
    assert_eq!(
        forms.len(),
        1,
        "#809 substrate parity contract must remain one self-contained Lisp data document"
    );
    let form = &forms[0];
    let exact_form_source = &source[form.span.start..form.span.end];
    let transport = format!(
        "(def life-substrate-parity-document (quote {exact_form_source}))"
    );
    eval_program(&transport, session)
        .expect("host observer must transport #809 contract bytes into Lisp data");
}

fn load_life_substrate_parity_witness(session: &mut Session) {
    let source = fs::read_to_string(repo_file(
        "tests/fixtures/life-substrate-parity-809-witness.lisp",
    ))
    .expect("#809 requires its Lisp-owned substrate parity witness");
    eval_program(&source, session).expect("life-substrate-parity-809-witness.lisp must load");
}

fn escape_lisp_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn actual_form_from_native(row: &WitnessRow, session: &mut Session) -> String {
    match eval_program(&row.expr, session) {
        Ok(result) => format!(
            "(value \"{}\")",
            escape_lisp_string(&result.value.to_string())
        ),
        Err(error) => format!("(error \"{:?}\")", error.kind),
    }
}

fn assert_lisp_owned_verdict_passes(session: &mut Session, row: &WitnessRow, actual: &str) {
    let program = format!(
        "(witness-pass? (witness-verdict (quote {}) (quote {})))",
        row.source, actual
    );
    let result = eval_program(&program, session)
        .unwrap_or_else(|error| panic!("witness verdict failed for {}: {error}", row.expr));
    assert_eq!(
        result.value.to_string(),
        "t",
        "Lisp-owned witness verdict rejected actual outcome for {}",
        row.expr
    );
}

fn init_meta_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    load_meta_evaluator_library(&mut session).expect("meta evaluator");
    load_witness_library(&mut session);
    eval_program(
        include_str!("../../../lib/generated/meta-semantic-registry.lisp"),
        &mut session,
    )
    .expect("generated semantic registry");
    eval_program("(def --witness-meta-env-- (quote ()))", &mut session)
        .expect("meta environment init");
    session
}

fn meta_verdict(session: &mut Session, row: &WitnessRow) -> String {
    let expr = escape_lisp_string(&row.expr);
    eval_program(
        &format!(
            "(def --witness-meta-step-- (my-eval-program (read-all \"{expr}\") --witness-meta-env--))"
        ),
        session,
    )
    .expect("meta step");
    eval_program(
        "(def --witness-meta-env-- (car --witness-meta-step--))",
        session,
    )
    .expect("thread meta environment");

    let meta_value = eval_program("(cdr --witness-meta-step--)", session)
        .expect("meta value transport")
        .value;
    let presented = escape_lisp_string(&meta_value.to_string());
    let program = format!(
        "(witness-verdict (quote {}) (witness-meta-outcome-presented (cdr --witness-meta-step--) \"{}\"))",
        row.source, presented
    );
    eval_program(&program, session)
        .unwrap_or_else(|error| panic!("meta witness verdict failed for {}: {error}", row.expr))
        .value
        .to_string()
}

#[test]
fn compiler_corpus_native_actuals_are_judged_only_by_lisp_owned_witness_logic() {
    let rows: Vec<_> = witness_rows()
        .into_iter()
        .filter(|row| row.compiler_corpus)
        .collect();
    assert!(!rows.is_empty(), "compiler-corpus must remain non-empty");

    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    load_witness_library(&mut session);
    let mut named_errors = 0usize;

    for row in &rows {
        let actual = actual_form_from_native(row, &mut session);
        assert_lisp_owned_verdict_passes(&mut session, row, &actual);
        if row.error.is_some() {
            named_errors += 1;
        }
    }

    assert!(
        named_errors > 0,
        "#113 corpus slice must contain at least one Lisp-authored named error witness"
    );
    assert!(
        rows.iter().any(|row| row.expr.contains("lambda")),
        "#113 compiler slice must retain lambda/application or closure evidence"
    );
    assert!(
        rows.iter().any(|row| row.expr.contains("defmacro")),
        "#113 compiler slice must retain macro evidence"
    );
}

#[test]
fn exact_d3_predicate_boundaries_follow_lisp_owned_bit_cases() {
    let rows = d3_predicate_boundary_rows();
    assert!(
        rows.len() >= 8,
        "typed D3 predicate boundary corpus must keep both polarities and peer surfaces"
    );

    for row in rows {
        let mut session = Session::default();
        load_core_library(&mut session).expect("core library");
        let value = eval_program(&row.expr, &mut session)
            .unwrap_or_else(|error| panic!("{} failed to evaluate: {error}", row.case))
            .value;

        assert_eq!(
            value.as_predicate_bit(),
            Some(row.expected_bit),
            "Lisp-owned case {} must cross the exact D3 -> D1 boundary without Number/list/T/NIL coercion; value={value}",
            row.case
        );
    }
}

#[test]
fn canon_zero_empty_list_is_data_not_false() {
    let rows = canon_zero_rows();
    assert!(
        !rows.is_empty(),
        "#215 active Canon 0 witness slice must remain non-empty"
    );

    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    load_witness_library(&mut session);

    for row in &rows {
        let actual = actual_form_from_native(row, &mut session);
        assert_lisp_owned_verdict_passes(&mut session, row, &actual);
    }
}

#[test]
fn structure_core_stays_stable_across_native_and_meta_eval() {
    let rows = structure_core_rows();
    assert!(
        !rows.is_empty(),
        "#230 structure-core witness slice must remain non-empty"
    );

    let mut native = Session::default();
    load_core_library(&mut native).expect("core library");
    load_witness_library(&mut native);

    let mut meta = init_meta_session();

    for row in &rows {
        let native_actual = actual_form_from_native(row, &mut native);
        assert_lisp_owned_verdict_passes(&mut native, row, &native_actual);

        let meta_result = meta_verdict(&mut meta, row);
        assert!(
            meta_result.starts_with("(witness-result (status pass)"),
            "#230 meta-eval disagreed with Lisp-owned structure row {}: {meta_result}",
            row.expr
        );
    }
}

#[test]
fn same_committed_corpus_drives_meta_eval_for_rows_admitted_to_that_backend() {
    let rows: Vec<_> = witness_rows()
        .into_iter()
        .filter(|row| row.meta_eval)
        .collect();
    assert!(
        !rows.is_empty(),
        "meta-eval witness slice must remain non-empty"
    );

    let mut session = init_meta_session();
    let mut checked_values = 0usize;

    for row in &rows {
        let verdict = meta_verdict(&mut session, row);
        assert!(
            verdict.starts_with("(witness-result (status pass)"),
            "meta-eval disagreed with Lisp-owned witness row {}: {verdict}",
            row.expr
        );
        if row.expected.is_some() {
            checked_values += 1;
        }
    }

    assert!(
        checked_values > 0,
        "meta witness slice must contain value witnesses"
    );
    // A class counts by its SENS code or, for rows not yet migrated, its name.
    for (required_head, code) in [
        ("quote", "00000001"),
        ("atom", "00000010"),
        ("eq", "00000011"),
        ("car", "00000101"),
        ("cdr", "00000110"),
        ("cons", "00000100"),
        ("cond", "00000111"),
    ] {
        let by_code = format!("({code}");
        let by_name = format!("({required_head} ");
        assert!(
            rows.iter().any(|row| {
                let expr = row.expr.trim_start();
                expr.starts_with(&by_code) || expr.starts_with(&by_name)
            }),
            "meta witness slice lost McCarthy-7/Canon-0 class `{required_head}` ({code})"
        );
    }
    assert!(
        rows.iter()
            .any(|row| row.expr.trim_start().starts_with("((lambda")),
        "meta witness slice must contain lambda application"
    );
}

#[test]
fn malformed_witness_fails_closed_as_lisp_data() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    load_witness_library(&mut session);

    let verdict = eval_program(
        "(witness-verdict (quote ((expr . \"(+ 1 2)\"))) (quote (value \"3\")))",
        &mut session,
    )
    .expect("malformed witness must return a named data verdict, not crash")
    .value
    .to_string();

    assert!(
        verdict.starts_with("(witness-result (status malformed)"),
        "missing expected/error must fail closed: {verdict}"
    );
}

#[test]
fn island_compat_semantics_are_owned_by_lisp_not_kernel_adapters() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    transport_island_compat_document(&mut session);
    load_island_compat_witness(&mut session);

    let verdict = eval_program("(island-compat-witness)", &mut session)
        .expect("Lisp-owned island compatibility witness must execute")
        .value
        .to_string();

    assert!(
        verdict.starts_with("(island-compat-witness (status pass)"),
        "Lisp-owned #749 island compatibility witness rejected the contract: {verdict}"
    );
}

#[test]
fn life_1_liveness_semantics_are_owned_by_lisp_data() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    transport_life_1_document(&mut session);
    load_life_1_contract_witness(&mut session);

    let verdict = eval_program("(life-1-contract-witness)", &mut session)
        .expect("Lisp-owned LIFE-1 contract witness must execute")
        .value
        .to_string();

    assert!(
        verdict.starts_with("(life-1-contract-witness (status pass)"),
        "Lisp-owned #785 LIFE-1 witness rejected the contract: {verdict}"
    );
}

#[test]
fn life_orchestration_parity_stays_fail_closed_without_real_graal_evidence() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    transport_life_substrate_parity_document(&mut session);
    load_life_substrate_parity_witness(&mut session);

    let verdict = eval_program("(life-substrate-parity-witness)", &mut session)
        .expect("Lisp-owned #809 witness must execute")
        .value
        .to_string();

    assert!(
        verdict.starts_with("(life-substrate-parity-witness (status pass)"),
        "Lisp-owned #809 substrate parity witness rejected its fail-closed contract: {verdict}"
    );

    let contract = fs::read_to_string(repo_file("contracts/life-substrate-parity-809.lisp"))
        .expect("#809 contract");
    let rows = contract
        .split("(rows")
        .nth(1)
        .expect("#809 contract must expose rows section");
    assert!(
        !rows.contains("(row "),
        "#809 must not claim GREEN substrate parity before wsm-graalvm#93 supplies real evidence"
    );
}

#[test]
fn semantic_ownership_audit_733_is_well_formed_lisp_inventory() {
    let source = fs::read_to_string(repo_file("contracts/semantic-ownership-audit-733.lisp"))
        .expect("#733 requires contracts/semantic-ownership-audit-733.lisp");
    let forms =
        parse(&source).expect("semantic-ownership-audit-733.lisp must be readable Lisp data");
    assert_eq!(forms.len(), 1, "audit inventory must be a single root form");

    let form = &forms[0];
    let crate::ExprKind::List(items) = &form.kind else {
        panic!("root form must be a list");
    };
    assert!(!items.is_empty(), "root form cannot be empty");
    let crate::ExprKind::Symbol(tag) = &items[0].kind else {
        panic!("root form must start with a tag symbol");
    };
    assert_eq!(&**tag, "semantic-ownership-audit/1");

    let mut semantic_authority_count = 0;
    let mut semantic_witness_count = 0;
    let mut execution_mechanism_count = 0;
    let mut unknown_count = 0;

    for entry_expr in items[1..].iter() {
        let crate::ExprKind::List(fields) = &entry_expr.kind else {
            panic!("each inventory entry must be an alist");
        };
        let mut category = None;
        for field_expr in fields.iter() {
            let crate::ExprKind::Pair(car, cdr) = &field_expr.kind else {
                continue;
            };
            if let crate::ExprKind::Symbol(k) = &car.kind {
                if &**k == "category" {
                    if let crate::ExprKind::Symbol(cat) = &cdr.kind {
                        category = Some(cat.to_string());
                    }
                }
            }
        }
        match category.as_deref() {
            Some("semantic-authority") => semantic_authority_count += 1,
            Some("semantic-witness") => semantic_witness_count += 1,
            Some("execution-mechanism") => execution_mechanism_count += 1,
            Some("unknown") => unknown_count += 1,
            _ => {}
        }
    }

    assert!(
        semantic_authority_count + semantic_witness_count >= 10,
        "audit must identify at least 10 semantic authority/witness items (found {})",
        semantic_authority_count + semantic_witness_count
    );
    assert!(
        execution_mechanism_count >= 10,
        "audit must identify at least 10 execution mechanism candidates (found {execution_mechanism_count})"
    );
    assert!(
        unknown_count >= 1,
        "audit must identify ambiguous items that require experiment before moving (found {unknown_count})"
    );
}

#[test]
fn primitive_budget_audit_734_accounts_for_197_ids_under_256_constraint() {
    let source = fs::read_to_string(repo_file("contracts/primitive-budget-audit-734.lisp"))
        .expect("#734 requires contracts/primitive-budget-audit-734.lisp");
    let forms = parse(&source).expect("primitive-budget-audit-734.lisp must be readable Lisp data");
    assert_eq!(forms.len(), 1, "audit inventory must be a single root form");

    let form = &forms[0];
    let crate::ExprKind::List(items) = &form.kind else {
        panic!("root form must be a list");
    };
    assert!(!items.is_empty(), "root form cannot be empty");
    let crate::ExprKind::Symbol(tag) = &items[0].kind else {
        panic!("root form must start with a tag symbol");
    };
    assert_eq!(&**tag, "primitive-budget-audit/1");

    let mut total_ids = 0;
    let mut reclaim_candidates = 0;
    let mut has_second = false;
    let mut has_cadr = false;
    let mut fourth_is_reclaimable = false;

    for entry_expr in items[1..].iter() {
        let crate::ExprKind::List(fields) = &entry_expr.kind else {
            panic!("each inventory entry must be an alist");
        };
        total_ids += 1;
        let mut name = String::new();
        let mut reclaim = false;

        for field_expr in fields.iter() {
            let crate::ExprKind::Pair(car, cdr) = &field_expr.kind else {
                continue;
            };
            if let crate::ExprKind::Symbol(k) = &car.kind {
                if &**k == "name" {
                    if let crate::ExprKind::String(n) = &cdr.kind {
                        name = n.to_string();
                    } else if let crate::ExprKind::Symbol(n) = &cdr.kind {
                        name = n.to_string();
                    }
                } else if &**k == "reclaim-candidate?" {
                    if let crate::ExprKind::Symbol(r) = &cdr.kind {
                        if &**r == "yes" {
                            reclaim = true;
                            reclaim_candidates += 1;
                        }
                    }
                }
            }
        }

        if name == "second" {
            has_second = true;
        } else if name == "cadr" {
            has_cadr = true;
        } else if name == "fourth" && reclaim {
            fourth_is_reclaimable = true;
        }
    }

    assert_eq!(
        total_ids, 197,
        "audit must account for all 197 experimental IDs"
    );
    assert!(
        total_ids <= 256,
        "total IDs must satisfy <= 256 hard budget"
    );
    assert!(
        reclaim_candidates > 0,
        "audit must identify reclaim candidates"
    );
    assert!(
        has_second && has_cadr,
        "second and cadr must both be present and distinct"
    );
    assert!(
        fourth_is_reclaimable,
        "fourth must be identified as reclaim candidate"
    );
}

// #735/#853 — SID → kernel semantic witnesses and opaque ABI probes are
// deliberately separate evidence classes.  A matching u8 is provenance, not
// proof that the kernel executed the registry identity's semantics.
#[test]
fn sid_kernel_witness_735_separates_semantic_execution_from_opaque_transport() {
    let src = std::fs::read_to_string(repo_file("contracts/sid-kernel-witness-735.lisp"))
        .expect("contracts/sid-kernel-witness-735.lisp must exist");
    let exprs = parse(&src).expect("contract must parse as valid Lisp");
    assert!(
        exprs.len() >= 2,
        "contract must contain semantic-witness map and opaque-probe evidence"
    );

    let map_items = match &exprs[0].kind {
        ExprKind::List(items) => items.clone(),
        _ => panic!("semantic witness map must be a list"),
    };
    assert!(matches!(
        &map_items[0].kind,
        ExprKind::Symbol(s) if &**s == "sid-kernel-witness-map/1"
    ));

    let mut seen_sids = std::collections::HashSet::new();
    let mut semantic_kernels = std::collections::HashSet::new();
    let mut live_count = 0usize;

    for entry in &map_items[1..] {
        let fields = match &entry.kind {
            ExprKind::List(items) => items.clone(),
            _ => panic!("each semantic map entry must be a list"),
        };
        assert!(matches!(
            &fields[0].kind,
            ExprKind::Symbol(s) if &**s == "sid-witness"
        ));

        let mut sid: Option<Sens8> = None;
        let mut witnesses: Vec<Expr> = Vec::new();
        for field in &fields[1..] {
            match &field.kind {
                ExprKind::Pair(key, value)
                    if matches!(&key.kind, ExprKind::Symbol(s) if &**s == "sid") =>
                {
                    sid = Some(match &value.kind {
                        ExprKind::Sid(sid) => *sid,
                        other => panic!("semantic sid must be exact bare Sens8, got {other:?}"),
                    });
                }
                ExprKind::List(items)
                    if !items.is_empty()
                        && matches!(&items[0].kind, ExprKind::Symbol(s) if &**s == "witnesses") =>
                {
                    witnesses = items[1..].to_vec();
                }
                _ => {}
            }
        }

        let sid = sid.expect("every semantic witness row needs a SID");
        assert!(seen_sids.insert(sid), "duplicate SID {sid}");

        for witness in witnesses {
            let fields = match witness.kind {
                ExprKind::List(items) => items,
                _ => continue,
            };
            if fields.is_empty()
                || !matches!(&fields[0].kind, ExprKind::Symbol(s) if &**s == "witness")
            {
                continue;
            }

            let mut kernel: Option<String> = None;
            let mut status: Option<String> = None;
            let mut probe_id: Option<Sens8> = None;
            let mut evidence_class: Option<String> = None;
            let mut evidence: Option<String> = None;

            for field in &fields[1..] {
                if let ExprKind::Pair(key, value) = &field.kind {
                    let key = match &key.kind {
                        ExprKind::Symbol(s) => &**s,
                        _ => continue,
                    };
                    match key {
                        "kernel" => {
                            if let ExprKind::Symbol(s) = &value.kind {
                                kernel = Some(s.to_string());
                            }
                        }
                        "status" => {
                            if let ExprKind::Symbol(s) = &value.kind {
                                status = Some(s.to_string());
                            }
                        }
                        "probe-id" => {
                            probe_id = Some(match &value.kind {
                                ExprKind::Sid(sid) => *sid,
                                other => panic!(
                                    "semantic witness probe-id must be exact bare Sens8, got {other:?}"
                                ),
                            });
                        }
                        "evidence-class" => {
                            if let ExprKind::Symbol(s) = &value.kind {
                                evidence_class = Some(s.to_string());
                            }
                        }
                        "evidence" => {
                            if let ExprKind::String(s) = &value.kind {
                                evidence = Some(s.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }

            let kernel = kernel.expect("semantic witness needs a kernel");
            semantic_kernels.insert(kernel.clone());
            if status.as_deref() == Some("live") {
                live_count += 1;
            }

            if matches!(kernel.as_str(), "prolog" | "datalog" | "clips")
                && status.as_deref() == Some("live")
            {
                assert_eq!(
                    evidence_class.as_deref(),
                    Some("semantic-execution"),
                    "external live semantic witness must explicitly prove semantic execution"
                );
                assert_eq!(
                    probe_id,
                    Some(sid),
                    "semantic execution witness must intentionally receive the mapped SID"
                );
                let evidence = evidence.expect("semantic execution witness needs evidence");
                assert!(
                    repo_file(&evidence).is_file(),
                    "semantic witness evidence must exist: {evidence}"
                );
            }
        }
    }

    assert!(semantic_kernels.contains("sens"));
    assert!(semantic_kernels.contains("common-lisp"));
    assert!(live_count > 0);
    assert!(seen_sids.len() >= 4);

    let probe_items = match &exprs[1].kind {
        ExprKind::List(items) => items.clone(),
        _ => panic!("opaque probe evidence must be a list"),
    };
    assert!(matches!(
        &probe_items[0].kind,
        ExprKind::Symbol(s) if &**s == "opaque-kernel-probe-evidence/1"
    ));

    let mut probe_kernels = std::collections::HashSet::new();
    for probe in &probe_items[1..] {
        let fields = match &probe.kind {
            ExprKind::List(items) => items,
            _ => panic!("probe row must be a list"),
        };
        assert!(matches!(
            &fields[0].kind,
            ExprKind::Symbol(s) if &**s == "probe"
        ));

        let mut kernel: Option<String> = None;
        let mut probe_id: Option<String> = None;
        let mut semantic_witness: Option<String> = None;
        let mut evidence: Option<String> = None;

        for field in &fields[1..] {
            if let ExprKind::Pair(key, value) = &field.kind {
                let key = match &key.kind {
                    ExprKind::Symbol(s) => &**s,
                    _ => continue,
                };
                match key {
                    "kernel" => {
                        if let ExprKind::Symbol(s) = &value.kind {
                            kernel = Some(s.to_string());
                        }
                    }
                    "probe-id" => {
                        if let ExprKind::String(s) = &value.kind {
                            probe_id = Some(s.to_string());
                        }
                    }
                    "semantic-witness" => {
                        if let ExprKind::Symbol(s) = &value.kind {
                            semantic_witness = Some(s.to_string());
                        }
                    }
                    "evidence" => {
                        if let ExprKind::String(s) = &value.kind {
                            evidence = Some(s.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        let kernel = kernel.expect("probe row needs kernel");
        let probe_id = probe_id.expect("probe row needs opaque byte");
        assert_eq!(probe_id.len(), 8);
        assert!(probe_id.chars().all(|ch| ch == '0' || ch == '1'));
        assert_eq!(
            semantic_witness.as_deref(),
            Some("no"),
            "opaque transport probes must never masquerade as semantic witnesses"
        );
        let evidence = evidence.expect("opaque probe needs mechanism evidence");
        assert!(repo_file(&evidence).is_file(), "missing probe evidence: {evidence}");
        probe_kernels.insert(kernel);
    }

    assert_eq!(
        probe_kernels,
        std::collections::HashSet::from([
            "prolog".to_string(),
            "datalog".to_string(),
            "clips".to_string(),
        ]),
        "all current opaque ABI probes must remain visible as transport evidence"
    );
}
