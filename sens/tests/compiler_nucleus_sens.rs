//! sens#3803 — executable N0 compiler nucleus.
//!
//! This is intentionally a bootstrap/differential witness, not the final
//! self-host proof.  The authority projection is generated from the one
//! production SENS role projection.  The SENS program performs generic exact
//! DomainIdentity lookup and never contains its own D3 bits->meaning table.
//! sens#3806 tracks moving the projection construction itself into SENS.

use sens::syntax::{Expr, ExprKind, Span};
use sens::{
    compiler_execution_role, eval_parsed_expressions, load_core_library,
    lower_program, parse_mixed_exact_domain, Bija3, Bit3, CompilerExecutionRole, CoreDomainIdentity,
    DomainIdentity, Session, Value,
};
use std::rc::Rc;

const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");

fn span() -> Span {
    Span::default()
}

fn d3_core(raw: u8) -> CoreDomainIdentity {
    CoreDomainIdentity::D3(Bija3::from_word(Bit3::new(raw).expect("D3 word")))
}

fn d3_value(raw: u8) -> DomainIdentity {
    DomainIdentity::D3(Bija3::from_word(Bit3::new(raw).expect("D3 word")))
}

fn symbol(name: &str) -> Expr {
    Expr {
        kind: ExprKind::Symbol(name.into()),
        span: span(),
    }
}

fn domain(identity: DomainIdentity) -> Expr {
    Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: span(),
    }
}

fn list(items: Vec<Expr>) -> Expr {
    Expr {
        kind: ExprKind::List(Rc::from(items.into_boxed_slice())),
        span: span(),
    }
}

fn quote(expr: Expr) -> Expr {
    Expr {
        kind: ExprKind::DomainCall(
            d3_core(0b001),
            Rc::from(vec![expr].into_boxed_slice()),
        ),
        span: span(),
    }
}

fn role_tag(role: CompilerExecutionRole) -> &'static str {
    match role {
        CompilerExecutionRole::SelectorHead => "selector-head",
        CompilerExecutionRole::SelectorTail => "selector-tail",
        CompilerExecutionRole::PairConstruct => "pair-construct",
    }
}

fn authority_projection() -> Expr {
    let rows = (0u8..=0b111)
        .filter_map(|raw| {
            let role = compiler_execution_role(d3_core(raw))?;
            Some(list(vec![
                domain(d3_value(raw)),
                symbol(role_tag(role)),
                symbol("bootstrap-proof"),
                symbol("bootstrap-provenance"),
            ]))
        })
        .collect();
    list(rows)
}

fn call_nucleus(identity: DomainIdentity) -> Expr {
    list(vec![
        symbol("compiler-nucleus"),
        domain(identity),
        quote(authority_projection()),
    ])
}

fn legacy_source_context(expr: &Expr, top_level_index: usize) -> String {
    let excerpt = NUCLEUS
        .get(expr.span.start..expr.span.end)
        .unwrap_or("<фрагмент поза межами джерела>");
    format!(
        "lib/compiler-nucleus.lisp, верхньорівнева форма {top_level_index}, байти {}..{}, фрагмент {excerpt:?}",
        expr.span.start, expr.span.end
    )
}

fn assert_no_legacy_identity(expr: &Expr, top_level_index: usize) {
    match &expr.kind {
        ExprKind::Sid(sid) => panic!(
            "legacy Sid entered SENS compiler nucleus: {sid}; {}",
            legacy_source_context(expr, top_level_index)
        ),
        ExprKind::Call(sid, _) => panic!(
            "legacy Call entered SENS compiler nucleus: {sid}; {}",
            legacy_source_context(expr, top_level_index)
        ),
        ExprKind::List(items) => {
            for item in items.iter() {
                assert_no_legacy_identity(item, top_level_index);
            }
        }
        ExprKind::Pair(head, tail) => {
            assert_no_legacy_identity(head, top_level_index);
            assert_no_legacy_identity(tail, top_level_index);
        }
        ExprKind::DomainCall(_, args) => {
            for arg in args.iter() {
                assert_no_legacy_identity(arg, top_level_index);
            }
        }
        ExprKind::Number(_, _)
        | ExprKind::Rational(_)
        | ExprKind::BinaryNumber(_)
        | ExprKind::NumericBuffer(_)
        | ExprKind::DomainIdentity(_)
        | ExprKind::String(_)
        | ExprKind::Symbol(_)
        | ExprKind::Local { .. } => {}
    }
}

fn list_values(value: &Value) -> Option<Vec<&Value>> {
    let mut out = Vec::new();
    let mut cursor = value;
    loop {
        match cursor {
            Value::Nil => return Some(out),
            Value::Pair(head, tail) => {
                out.push(head.as_ref());
                cursor = tail.as_ref();
            }
            _ => return None,
        }
    }
}

#[test]
fn nucleus_source_lowers_without_legacy_sid_or_call_nodes() {
    let parsed = parse_mixed_exact_domain(NUCLEUS).expect("compiler nucleus source parses");
    let lowered = lower_program(&parsed);
    assert!(lowered.len() >= 2, "compiler nucleus contains executable language definitions");
    for (top_level_index, expression) in lowered.iter().enumerate() {
        assert_no_legacy_identity(expression, top_level_index);
    }
}


#[test]
fn nucleus_top_level_form_failure_is_localized() {
    let expressions =
        parse_mixed_exact_domain(NUCLEUS).expect("compiler nucleus source parses");
    let mut session = Session::default();
    load_core_library(&mut session).expect("active core loads");

    for (index, expression) in expressions.iter().enumerate() {
        if let Err(error) =
            eval_parsed_expressions(std::slice::from_ref(expression), &mut session)
        {
            panic!(
                "compiler nucleus top-level form {index} failed: {error:?}; expression={expression:?}"
            );
        }
    }
}


#[test]
fn nucleus_looks_up_current_d3_roles_by_exact_identity_and_fails_closed() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("active core loads");
    let expressions = parse_mixed_exact_domain(NUCLEUS).expect("compiler nucleus parses");
    eval_parsed_expressions(&expressions, &mut session)
        .expect("SENS compiler nucleus loads");

    for raw in 0u8..=0b111 {
        let expected_role = compiler_execution_role(d3_core(raw));
        let result = eval_parsed_expressions(&[call_nucleus(d3_value(raw))], &mut session)
            .unwrap_or_else(|error| panic!("D3:{raw:03b} nucleus call failed: {error:?}"))
            .value;

        match expected_role {
            None => assert!(
                matches!(result, Value::Nil),
                "D3:{raw:03b} is outside the bounded compiler role projection and must fail closed"
            ),
            Some(role) => {
                let row = list_values(&result).expect("authority hit returns one proper row");
                assert_eq!(row.len(), 4, "normalized authority row stays four fields");
                assert_eq!(row[0], &Value::DomainIdentity(d3_value(raw)));
                assert!(
                    matches!(row[1], Value::Symbol(name) if name.as_ref() == role_tag(role)),
                    "D3:{raw:03b} carries the bootstrap-projected abstract role"
                );
                assert!(
                    matches!(row[2], Value::Symbol(name) if name.as_ref() == "bootstrap-proof")
                );
                assert!(
                    matches!(row[3], Value::Symbol(name) if name.as_ref() == "bootstrap-provenance")
                );
            }
        }
    }
}


const COMPILER_VARIADIC_D1_LAW: &str =
    include_str!("../../../tests/semantic/compiler-variadic-tail-d1-law.lisp");
const COMPILER_VARIADIC_D1_MUTANT: &str =
    include_str!("../../../tests/semantic/compiler-variadic-tail-d1-mutant.lisp");

const COMPILER_ARITY_D1_LAW: &str =
    include_str!("../../../tests/semantic/compiler-arity-proper-list-law.lisp");
const COMPILER_ARITY_D1_MUTANT: &str =
    include_str!("../../../tests/semantic/compiler-arity-proper-list-mutant.lisp");

const COMPILER_PROGRAM_LIST_D1_LAW: &str =
    include_str!("../../../tests/semantic/compiler-program-list-d1-law.lisp");
const COMPILER_PROGRAM_LIST_D1_MUTANT: &str =
    include_str!("../../../tests/semantic/compiler-program-list-d1-mutant.lisp");

/// Чотири точні D1-вердикти обчислює двійковий SENS, а не Rust.
#[test]
fn compiler_program_list_fail_closed_is_sens_owned() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("active core");
    let nucleus = parse_mixed_exact_domain(NUCLEUS).expect("exact nucleus");
    eval_parsed_expressions(&nucleus, &mut session).expect("nucleus loads");

    let forms = parse_mixed_exact_domain(COMPILER_PROGRAM_LIST_D1_LAW)
        .expect("four executable SENS-owned exact D1/D3/D4 verdicts");
    assert_eq!(forms.len(), 4);
    for (index, form) in forms.iter().enumerate() {
        let verdict = eval_parsed_expressions(std::slice::from_ref(form), &mut session)
            .unwrap_or_else(|error| panic!("SENS law {index} failed: {error}"))
            .value;
        assert_eq!(
            verdict.as_predicate_bit(),
            Some(true),
            "SENS law {index} must itself return exact D1:YES: {verdict}"
        );
    }

    // Однобітний mutant змінює виключно очікуване D1:NO на D1:YES.
    assert_eq!(
        COMPILER_PROGRAM_LIST_D1_LAW.replacen("#d1:0", "#d1:1", 1),
        COMPILER_PROGRAM_LIST_D1_MUTANT
    );
    let mutant = parse_mixed_exact_domain(COMPILER_PROGRAM_LIST_D1_MUTANT)
        .expect("mutated SENS source must remain syntactically valid");
    assert_eq!(mutant.len(), forms.len());
    assert!(
        eval_parsed_expressions(std::slice::from_ref(&mutant[0]), &mut session)
            .is_err(),
        "one-bit D1 mutation must select SENS-owned fail-closed CAR(QUOTE EMPTY)"
    );

    // Строгі arity і COND-clause закони визначені лише самими SENS-формами.
    let exact = parse_mixed_exact_domain(COMPILER_ARITY_D1_LAW)
        .expect("SENS exact D1/D3/D4 arity and proper-list law parses");
    assert_eq!(exact.len(), 14, "14 independent SENS self-verdicts");
    for (index, form) in exact.iter().enumerate() {
        let verdict = eval_parsed_expressions(std::slice::from_ref(form), &mut session)
            .unwrap_or_else(|error| panic!("SENS arity/proper-list law {index}: {error}"))
            .value;
        assert_eq!(
            verdict.as_predicate_bit(),
            Some(true),
            "SENS arity/proper-list self-verdict {index} was not exact D1:YES: {verdict}"
        );
    }

    // Лише один очікуваний біт D1:0 → D1:1; SENS сам викликає свій
    // fail-closed CAR(QUOTE EMPTY) і відхиляє неприпустимий mutant.
    assert_eq!(
        COMPILER_ARITY_D1_LAW.replacen("#d1:0", "#d1:1", 1),
        COMPILER_ARITY_D1_MUTANT
    );
    let mutant = parse_mixed_exact_domain(COMPILER_ARITY_D1_MUTANT)
        .expect("one-bit SENS mutant must parse");
    assert_eq!(mutant.len(), exact.len());
    assert!(
        eval_parsed_expressions(std::slice::from_ref(&mutant[0]), &mut session)
            .is_err(),
        "SENS must reject the false success of improper one-argument dotted tail"
    );

    // Всі expected/actual і закон списку належать SENS-коду.
    // Rust — тільки читач/виконавець 15 самодостатніх D1-вердиктів.
    let variadic = parse_mixed_exact_domain(COMPILER_VARIADIC_D1_LAW)
        .expect("exact SENS variadic proper-tail laws parse");
    assert_eq!(variadic.len(), 15, "15 SENS-owned variadic arity verdicts");
    for (index, form) in variadic.iter().enumerate() {
        let verdict = eval_parsed_expressions(std::slice::from_ref(form), &mut session)
            .unwrap_or_else(|error| panic!("SENS variadic tail law {index}: {error}"))
            .value;
        assert_eq!(
            verdict.as_predicate_bit(),
            Some(true),
            "SENS variadic tail self-verdict {index} was not D1:YES: {verdict}"
        );
    }
    assert_eq!(
        COMPILER_VARIADIC_D1_LAW.replacen("#d1:1", "#d1:0", 1),
        COMPILER_VARIADIC_D1_MUTANT,
        "mutant changes exactly one expected D1 bit"
    );
    let mutant = parse_mixed_exact_domain(COMPILER_VARIADIC_D1_MUTANT)
        .expect("one-bit SENS variadic mutant parses");
    assert_eq!(mutant.len(), variadic.len());
    assert!(
        eval_parsed_expressions(std::slice::from_ref(&mutant[0]), &mut session)
            .is_err(),
        "SENS must itself reject the one-bit variadic verdict mutant"
    );
}
