//! Mechanical runner for SENS-owned semantic witnesses.
//!
//! The semantic expectation is encoded inside each binary SENS program.
//! Rust loads admitted language libraries and reports only whether the
//! self-verdict program executes successfully. It contains no truth table,
//! expected result value, or duplicate language law.

use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source,
    parse_canonical_binary, parse_mixed_exact_domain_reasoning_source, Session,
};

const RESULT_STATUS: &str = include_str!("../../../lib/result-status.lisp");
const RESULT_STATUS_D1_WITNESS: &str =
    include_str!("../../../tests/semantic/result-status-d1-law.bits");

fn load_reasoning(session: &mut Session, path: &str, source: &str) {
    let parsed = parse_mixed_exact_domain_reasoning_source(path, source)
        .unwrap_or_else(|error| panic!("mechanical parse failed for {path}: {error}"));
    eval_parsed_expressions(&parsed, session)
        .unwrap_or_else(|error| panic!("mechanical library load failed for {path}: {error}"));
}

#[test]
fn result_status_exact_d1_binary_self_verdict() {
    assert!(
        RESULT_STATUS_D1_WITNESS
            .bytes()
            .all(|byte| matches!(byte, b'0' | b'1' | b' ' | b'\n' | b'\r' | b'\t')),
        "semantic witness must be exact binary SENS source"
    );

    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("mechanical Core loader must admit current source");
    load_reasoning(&mut session, "lib/result-status.lisp", RESULT_STATUS);

    let witness = parse_canonical_binary(RESULT_STATUS_D1_WITNESS)
        .expect("binary SENS witness must have canonical D2 framing");
    eval_parsed_expressions(&witness, &mut session)
        .expect("SENS-owned result-status self-verdict rejected");
}


const X86_ENCODING_DIRECT_D4_LAW: &str =
    include_str!("../../../tests/semantic/x86-encoder-let-d4-law.bits");
const X86_ENCODING_DIRECT_D4_MUTANT: &str =
    include_str!("../../../tests/semantic/x86-encoder-let-d4-mutant.bits");
/// Лише механічний транспорт: усі закони й вердикти закодовано самою
/// двійковою мовою SENS, а не таблицею очікувань у Rust або Python.
#[test]
fn x86_encoding_let_to_exact_d4_binary_sens_verdict() {
    for source in [X86_ENCODING_DIRECT_D4_LAW, X86_ENCODING_DIRECT_D4_MUTANT] {
        assert!(source.bytes().all(|byte|
            matches!(byte, b'0' | b'1' | b' ' | b'\n' | b'\r' | b'\t')
        ), "SENS source must consist only of bits and separators");
    }

    // All semantic expectations live in the executable, binary SENS
    // D3 COND programs: wrong results select a forbidden D1 call head.
    // Rust's sole authority is that the real evaluator runs the program.
    let forms = parse_canonical_binary(X86_ENCODING_DIRECT_D4_LAW)
        .expect("canonical D1/D2/D3/D4/D7 framing");
    assert_eq!(forms.len(), 4, "four SENS-authored verdict programs");
    for (index, form) in forms.iter().enumerate() {
        eval_parsed_expressions(std::slice::from_ref(form), &mut Session::default())
            .unwrap_or_else(|error| panic!("SENS law {} rejected: {error}", index + 1));
    }

    // Transport-only one-bit mutant assertion: SENS itself must reject the
    // wrong branch. Rust does NOT compare a language truth value to YES/NO.
    let good: Vec<_> = X86_ENCODING_DIRECT_D4_LAW.lines().next()
        .expect("first binary verdict").split_whitespace().collect();
    let bad: Vec<_> = X86_ENCODING_DIRECT_D4_MUTANT.split_whitespace().collect();
    assert_eq!(good.len(), bad.len(), "mutation cannot change framing");
    let differences: Vec<_> = good.iter().zip(bad.iter())
        .filter(|(a, b)| a != b).collect();
    assert_eq!(differences, vec![(&"1", &"0")], "one D1 bit, no new width");

    let mutant = parse_canonical_binary(X86_ENCODING_DIRECT_D4_MUTANT)
        .expect("single-bit mutant is still canonical SENS");
    assert_eq!(mutant.len(), 1);
    assert!(eval_parsed_expressions(&mutant, &mut Session::default()).is_err(),
        "binary SENS must reject its own mutated invalid-D1 branch");
}

const META_EVAL_MUTUAL_D4_LAW: &str =
    include_str!("../../../tests/semantic/meta-eval-mutual-d4-law.bits");

/// Механічний транспорт: зміст перевірок і їхній очікуваний D1=1
/// належать виключно виконуваним бітовим формам SENS.
#[test]
fn meta_eval_mutual_d4_exact_sens_self_verdict() {
    assert!(
        META_EVAL_MUTUAL_D4_LAW
            .bytes()
            .all(|byte| matches!(byte, b'0' | b'1' | b' ' | b'\n' | b'\r' | b'\t')),
        "SENS must own all semantic assertions in exact binary source"
    );
    let forms = parse_canonical_binary(META_EVAL_MUTUAL_D4_LAW)
        .expect("canonical D2 framing and exact D1/D3/D4/D7 words");
    assert_eq!(forms.len(), 4, "one executable binary SENS self-proof per line");

    for (index, form) in forms.iter().enumerate() {
        eval_parsed_expressions(std::slice::from_ref(form), &mut Session::default())
            .unwrap_or_else(|error| panic!(
                "binary SENS self-proof {} rejected its own law: {error}", index + 1
            ));
    }
}

const EPISTEMIC_D1_SHORTENING_LAW: &str =
    include_str!("../../../tests/semantic/epistemic-d1-shortening-law.bits");

/// Rust тільки транспортує виконувані двійкові SENS-форми.
/// Рівність D3 ATOM(QUOTE empty) та D1:YES, негативна гілка і
/// вибір COND визначаються виключно SENS-програмою.
#[test]
fn epistemic_d1_shortening_binary_sens_self_verdict() {
    assert!(
        EPISTEMIC_D1_SHORTENING_LAW.bytes().all(|b|
            matches!(b, b'0' | b'1' | b' ' | b'\n' | b'\r' | b'\t')
        ),
        "SENS witness must contain only binary words and separators"
    );
    let forms = parse_canonical_binary(EPISTEMIC_D1_SHORTENING_LAW)
        .expect("canonical D2/D3/D1 program");
    for form in forms.iter() {
        let verdict = eval_parsed_expressions(
            std::slice::from_ref(form), &mut Session::default()
        ).expect("SENS-owned D1 law must execute").value;
        assert_eq!(
            verdict.as_predicate_bit(), Some(true),
            "binary SENS self-verdict is not D1:YES: {verdict}"
        );
    }
}

const EPISTEMIC_EQ_D1_LAW_MUTANT: &str =
    include_str!("../../../tests/semantic/epistemic-eq-d1-law-mutant.bits");

/// Двійкова SENS-програма сама визначає вердикт:
/// D3:010 ATOM(D3:001 QUOTE(D3:000 empty)) == D1:1.
/// Якщо єдина очікувана координата D1 змінена 1→0, SENS переходить
/// на недопущений виклик (D1:0); Rust лише відстежує результат виконання.
#[test]
fn epistemic_d3_eq_d1_law_and_one_bit_mutant() {
    assert!(
        EPISTEMIC_EQ_D1_LAW_MUTANT.bytes().all(|c|
            matches!(c, b'0' | b'1' | b' ' | b'\n' | b'\r' | b'\t')
        ),
        "epistemic self-verdict must be canonical binary SENS only"
    );

    let lines: Vec<&str> = EPISTEMIC_EQ_D1_LAW_MUTANT.lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 2, "one positive SENS law and its mutant");

    let law_tokens: Vec<&str> = lines[0].split_whitespace().collect();
    let mutant_tokens: Vec<&str> = lines[1].split_whitespace().collect();
    assert_eq!(law_tokens.len(), mutant_tokens.len(), "D2 framing preserved");
    let changes: Vec<(&str, &str)> = law_tokens.iter().zip(mutant_tokens.iter())
        .filter(|(left, right)| left != right)
        .map(|(left, right)| (*left, *right))
        .collect();
    assert_eq!(changes, vec![("1", "0")],
        "exactly one D1 bit changes; no domain-width substitution");

    let law = parse_canonical_binary(lines[0])
        .expect("binary epistemic D3/D1 law must parse");
    assert_eq!(law.len(), 1, "one executable D2-framed law");
    let value = eval_parsed_expressions(&law, &mut Session::default())
        .expect("SENS-owned positive law must execute")
        .value;
    assert!(value.as_predicate_bit().is_some(),
        "successful SENS self-verdict must remain D1, not NIL/data");

    let mutant = parse_canonical_binary(lines[1])
        .expect("one-bit mutant must retain canonical D2 framing");
    assert_eq!(mutant.len(), 1);
    let failure = eval_parsed_expressions(&mutant, &mut Session::default())
        .err()
        .expect("SENS must reject its own one-bit falsifier");
    assert!(
        failure.to_string().contains("domain identity is not callable under its ratified law: D1"),
        "negative witness failed for an unrelated cause: {failure}"
    );
    println!(
        "SENS_NATIVE_EPISTEMIC_D1 id=epistemic-eq-d1-law-mutant layer=eval positive=executed mutant=D1-noncallable fixture=tests/semantic/epistemic-eq-d1-law-mutant.bits"
    );
}
