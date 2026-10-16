//! Exercises lib/clips-import.my — Step 1 of a universal-ish importer for
//! old symbolic-AI systems: CLIPS's `deffacts` (facts only, no variables),
//! per PLAN.md's forward-chaining/CLIPS thread and NASA's Johnson Space
//! Center origin of CLIPS itself. Real CLIPS source parses cleanly as
//! ordinary sens data via `read`/`quote` — no dedicated tokenizer or
//! string primitives needed.
//! Pereviriaie lib/clips-import.my — Krok 1 universalnoho(-uvatoho)
//! importera zi starykh symvolnykh AI-system: CLIPS `deffacts` (fakty bez
//! zminnykh). Spravzhnii CLIPS-kod chysto parsytsia yak zvychaini sens dani
//! cherez `read`/`quote` — bez okremoho tokenizatora chy riadkovykh
//! prymityviv.
//! Prüft lib/clips-import.my — Schritt 1 eines universell(-ähnlichen)
//! Importers für alte symbolische KI-Systeme: CLIPS' `deffacts` (Fakten
//! ohne Variablen). Echter CLIPS-Quellcode parst sauber als gewöhnliche
//! sens-Daten über `read`/`quote` — kein eigener Tokenizer oder
//! String-Primitive nötig.

use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source, eval_pinned_reasoning_source,
    eval_program, parse_mixed_exact_domain, Session,
};

fn eval_import(source: &str) -> String {
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session,
    )
    .expect("test_id=clips-defrule-d4-split: exact-domain Core must load");
    for (path, library) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp")),
    ] {
        eval_pinned_reasoning_source(path, library, &mut session)
            .unwrap_or_else(|err| panic!("test_id=clips-defrule-d4-split: {path} failed to load: {err}"));
    }
    eval_pinned_reasoning_source(
        "lib/clips-import.lisp",
        include_str!("../../../lib/clips-import.lisp"),
        &mut session,
    )
    .expect("test_id=clips-defrule-d4-split: CLIPS library must load through pinned exact D9 reader");
    // Test inputs retain their historical data spelling. Only executable
    // library source is parsed as current exact-domain SENS.
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn clips_double_colon_prefix_is_an_exact_d1_total_predicate() {
    // The SENS library, not Rust, computes all predicates. One removed ':'
    // is the negative witness. Failure must be D1:NO, never the NIL datum.
    for (label, source, expected) in [
        ("positive", "(clips-string-starts-with-double-colon? \"::sensor\")", "1"),
        ("one-character-mutant", "(clips-string-starts-with-double-colon? \":sensor\")", "0"),
        ("non-prefix", "(clips-string-starts-with-double-colon? \"sensor::value\")", "0"),
        ("empty", "(clips-string-starts-with-double-colon? \"\")", "0"),
        ("short", "(clips-string-starts-with-double-colon? \":\")", "0"),
    ] {
        let actual = eval_import(source);
        assert_eq!(
            actual, expected,
            "test_id=clips-double-colon-d1 layer=eval SENS output={actual} case={label}"
        );
    }
}

#[test]
fn clips_defrule_exact_pipeline_probes() {
    // Diagnostic transport only: every intermediate value is computed by SENS.
    // These probes locate the first real failure without inventing a Rust
    // replacement for the SENS parser, evaluator or CLIPS rules.
    let samples = [
        (
            "split",
            "(clips-split-at-arrow (quote ((planet ?x) => (assert (has-mass ?x)))))",
        ),
        (
            "all-asserts",
            "(clips-all-asserts? (quote ((assert (has-mass ?x)))))",
        ),
        (
            "conclusions",
            "(clips-assert-conclusions (quote ((assert (has-mass ?x)))))",
        ),
        (
            "preamble",
            "(clips-strip-rule-preamble (quote ((planet ?x) => (assert (has-mass ?x)))))",
        ),
        (
            "conditions",
            "(clips-convert-vars (quote ((planet ?x))))",
        ),
        (
            "templates",
            "(clips-convert-template-list (quote ((planet ?x))) (quote ()))",
        ),
        (
            "from-parts",
            "(clips-defrule-clauses-from-parts (quote ((planet (var x)))) (quote ((assert (has-mass ?x)))) (quote ()))",
        ),
        (
            "defrule",
            "(clips-defrule->clauses (quote (defrule mass-rule (planet ?x) => (assert (has-mass ?x)))) (quote ()))",
        ),
    ];
    let mut seen = Vec::new();
    for (label, source) in samples {
        let result = eval_import(source);
        println!("SENS_CLIPS_INTERMEDIATE test_id=clips-defrule-d4-split step={label} actual={result}");
        seen.push((label, result));
    }
    for (label, result) in seen {
        if label != "all-asserts" {
            assert_ne!(
                result, "()",
                "test_id=clips-defrule-d4-split layer=eval step={label} returned empty SENS value"
            );
        }
    }
}

#[test]
fn clips_deffacts_becomes_zero_condition_clauses() {
    // A real CLIPS `deffacts` block, read as data rather than evaluated.
    let source = r#"
        (clips-import (quote ((deffacts initial-facts (planet earth) (planet mars) (star sun)))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((planet earth)) ((planet mars)) ((star sun)))"
    );
}

#[test]
fn clips_import_skips_unsupported_forms_without_erroring() {
    // deffunction isn't supported (no step covers it) — a mixed file
    // still imports whatever it can, rather than failing the whole import.
    let source = r#"
        (clips-import (quote (
            (deffacts initial-facts (planet earth))
            (deffunction square (?x) (* ?x ?x))
        )))
    "#;
    assert_eq!(eval_import(source), "(((planet earth)))");
}

#[test]
fn clips_defrule_converts_question_mark_variables_to_var_terms() {
    let source = r#"
        (clips-import (quote ((defrule mass-rule (planet ?x) => (assert (has-mass ?x))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((has-mass (var x)) (planet (var x))))"
    );
}

#[test]
fn clips_defrule_leaves_non_variable_arguments_untouched() {
    // A CLIPS fact argument can be a plain symbol or a number, not just a
    // `?`-prefixed variable — clips-var? must not choke on either.
    let source = r#"
        (clips-import (quote ((defrule hot (temperature 98) => (assert (alert critical))))))
    "#;
    assert_eq!(eval_import(source), "(((alert critical) (temperature 98)))");
}

#[test]
fn clips_defrule_with_multiple_conditions_converts_all_of_them() {
    let source = r#"
        (clips-import (quote ((defrule grandparent-rule
                           (parent ?x ?z) (parent ?z ?y)
                           => (assert (grandparent ?x ?y))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((grandparent (var x) (var y)) (parent (var x) (var z)) (parent (var z) (var y))))"
    );
}

#[test]
fn clips_defrule_with_no_asserts_imports_as_no_clauses() {
    let source = r#"
        (clips-import (quote ((defrule broken (planet ?x) => (printout t "hello")))))
    "#;
    assert_eq!(eval_import(source), "()");
}

#[test]
fn clips_defrule_strips_a_docstring_before_its_conditions() {
    // Regression: an empty docstring "" right after the rule name used to
    // land in the condition list as a stray, never-matching pattern,
    // silently killing the whole rule.
    let source = r#"
        (clips-import (quote ((defrule mass-rule "explains why planets have mass"
                           (planet ?x) => (assert (has-mass ?x))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((has-mass (var x)) (planet (var x))))"
    );
}

#[test]
fn clips_defrule_strips_a_declare_salience_before_its_conditions() {
    let source = r#"
        (clips-import (quote ((defrule mass-rule (declare (salience 10))
                           (planet ?x) => (assert (has-mass ?x))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((has-mass (var x)) (planet (var x))))"
    );
}

#[test]
fn clips_defrule_strips_both_docstring_and_declare_together() {
    let source = r#"
        (clips-import (quote ((defrule mass-rule "docstring" (declare (salience 10))
                           (planet ?x) => (assert (has-mass ?x))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((has-mass (var x)) (planet (var x))))"
    );
}

#[test]
fn clips_defrule_with_a_multi_fact_assert_produces_one_clause_per_fact() {
    // Real CLIPS assert can take multiple facts in one call:
    // (assert (number 0) (number 1) (number 2)) is one assert, three
    // facts — found on a genuine external .clp file, not guessed.
    let source = r#"
        (clips-import (quote ((defrule init (foo) => (assert (number 0) (number 1) (number 2))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((number 0) (foo)) ((number 1) (foo)) ((number 2) (foo)))"
    );
}

#[test]
fn clips_defrule_with_multiple_asserts_produces_one_clause_per_assert() {
    // N assertions sharing one LHS become N clauses, each with the same
    // (converted) condition list — logically equivalent to CLIPS firing
    // all N assertions together whenever the shared conditions hold.
    let source = r#"
        (clips-import (quote ((defrule two-asserts (planet ?x) => (assert (a ?x)) (assert (b ?x))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((a (var x)) (planet (var x))) ((b (var x)) (planet (var x))))"
    );
}

#[test]
fn clips_import_mixes_deffacts_and_defrule_into_one_usable_module() {
    let source = r#"
        (def imported (clips-import (quote (
            (deffacts init (planet earth))
            (defrule mass-rule (planet ?x) => (assert (has-mass ?x)))
        ))))
        (defmodule imported-astro imported)
        (forward-in (quote imported-astro))
    "#;
    assert_eq!(eval_import(source), "((has-mass earth) (planet earth))");
}

#[test]
fn clips_defrule_with_a_not_condition_imports_and_runs_correctly() {
    // Was skipped entirely as of clips-import.my Step 5, back when
    // lib/forward.lisp's match-conditions had no negation-as-failure
    // handling. Step 6 added it (match-one-condition/
    // match-negated-condition), so this now imports normally and derives
    // exactly the right fact: tweety (animal, not penguin) becomes a
    // bird; pingu (a penguin) does not.
    let source = r#"
        (def imported (clips-import (quote (
            (deffacts init (animal tweety) (animal pingu) (penguin pingu))
            (defrule bird-rule (animal ?x) (not (penguin ?x)) => (assert (bird ?x)))
        ))))
        (defmodule zoo imported)
        (forward-in (quote zoo))
    "#;
    assert_eq!(
        eval_import(source),
        "((bird tweety) (penguin pingu) (animal pingu) (animal tweety))"
    );
}

#[test]
fn clips_defrule_with_printout_alongside_assert_still_imports_the_assert() {
    // Regression: a rule mixing printout (debug output) with assert used
    // to import as no clauses at all, silently dropping a perfectly
    // representable assertion just because printout sat next to it.
    let source = r#"
        (clips-import (quote ((defrule mass-rule (planet ?x)
            => (printout t "found planet " ?x crlf) (assert (has-mass ?x))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((has-mass (var x)) (planet (var x))))"
    );
}

#[test]
fn clips_defrule_with_retract_still_imports_as_no_clauses() {
    // retract refers to a fact by CLIPS fact-address, a concept this
    // project's set-of-facts model has no equivalent for — still
    // disqualifying, unlike the harmless printout above.
    let source = r#"
        (clips-import (quote ((defrule broken (planet ?x) => (retract ?x) (assert (has-mass ?x))))))
    "#;
    assert_eq!(eval_import(source), "()");
}

#[test]
fn clips_deftemplate_converts_named_slots_to_positional_order() {
    let source = r#"
        (clips-import (quote (
            (deftemplate reading (slot sensor) (slot value))
            (deffacts init (reading (value 98) (sensor probe1)))
        )))
    "#;
    assert_eq!(eval_import(source), "(((reading probe1 98)))");
}

#[test]
fn clips_deftemplate_slot_order_holds_regardless_of_slot_order_in_the_fact() {
    let source = r#"
        (clips-import (quote (
            (deftemplate reading (slot sensor) (slot value))
            (deffacts init (reading (sensor probe1) (value 98)))
        )))
    "#;
    assert_eq!(eval_import(source), "(((reading probe1 98)))");
}

#[test]
fn clips_deftemplate_applies_inside_defrule_conditions_and_conclusions() {
    let source = r#"
        (clips-import (quote (
            (deftemplate reading (slot sensor) (slot value))
            (defrule hot-rule (reading (sensor ?s) (value 98))
                => (assert (alert (sensor ?s))))
            (deftemplate alert (slot sensor))
        )))
    "#;
    assert_eq!(
        eval_import(source),
        "(((alert (var s)) (reading (var s) 98)))"
    );
}

#[test]
fn clips_deftemplate_works_end_to_end_through_forward_in() {
    let source = r#"
        (def imported (clips-import (quote (
            (deftemplate reading (slot sensor) (slot value))
            (deffacts init (reading (sensor probe1) (value 98)))
            (defrule hot-rule (reading (sensor ?s) (value 98)) => (assert (alert ?s)))
        ))))
        (defmodule sensors imported)
        (forward-in (quote sensors))
    "#;
    assert_eq!(eval_import(source), "((alert probe1) (reading probe1 98))");
}

#[test]
fn clips_deftemplate_converts_facts_nested_inside_or() {
    // Regression: clips-convert-template used to recurse through a `not`
    // wrapper only, so a template fact inside `or`/`and` never got its
    // named slots converted to positional form, silently never matching
    // anything.
    let source = r#"
        (def imported (clips-import (quote (
            (deftemplate cat (slot name))
            (deftemplate dog (slot name))
            (deffacts init (cat (name tom)) (dog (name rex)))
            (defrule pet-rule (or (cat (name ?x)) (dog (name ?x))) => (assert (pet ?x)))
        ))))
        (defmodule zoo imported)
        (forward-in (quote zoo))
    "#;
    assert_eq!(
        eval_import(source),
        "((pet rex) (pet tom) (dog rex) (cat tom))"
    );
}

#[test]
fn clips_import_result_feeds_straight_into_defmodule() {
    // The whole point: no hand-editing step between import and use.
    let source = r#"
        (def imported-clauses (clips-import (quote ((deffacts initial-facts (planet earth) (star sun))))))
        (defmodule imported imported-clauses)
        (reason-in (quote imported) (quote (planet earth)))
    "#;
    assert_eq!(
        eval_import(source),
        "((() (proved (planet earth) (planet earth) ())))"
    );
}

#[test]
fn clips_deftemplate_name_is_matched_regardless_of_a_defmodule_qualifier() {
    // Regression: real CLIPS files namespace deftemplate names with a
    // `defmodule` prefix (`QUESTIONS::question`), then assert plain
    // `(question ...)` facts elsewhere — CLIPS resolves that against
    // whichever module is current. This importer models no modules at
    // all, so both the template's own registered name and any fact head
    // referring to it must be compared with the `defmodule::` prefix
    // stripped, or the named-slot conversion silently never fires.
    let source = r#"
        (clips-import (quote (
            (deftemplate QUESTIONS::question (slot attribute) (slot text))
            (deffacts init (question (attribute color) (text "what color?")))
        )))
    "#;
    assert_eq!(eval_import(source), "(((question color \"what color?\")))");
}

#[test]
fn clips_condition_slot_with_no_value_does_not_crash_the_importer() {
    // Regression: a CLIPS condition can name a multislot with no value at
    // all, e.g. `(precursors)`, meaning "match regardless of what's in
    // this slot". clips-slot-value-of used to assume every slot form has
    // a value part and called `second` unconditionally, crashing with
    // "car expects a non-empty list" the first time a real file used this
    // shorthand.
    let source = r#"
        (clips-import (quote (
            (deftemplate question (slot attribute) (multislot precursors))
            (defrule ask (question (attribute ?a) (precursors)) => (assert (asked ?a)))
        )))
    "#;
    assert_eq!(
        eval_import(source),
        "(((asked (var a)) (question (var a) ())))"
    );
}

#[test]
fn clips_import_stays_stack_safe_on_a_deffacts_block_with_many_facts() {
    // Regression: clips-facts->clauses/clips-import-forms used to build
    // their result with `(cons ... (recurse ...))`/`(append ... (recurse
    // ...))`, neither a tail call, so the Rust stack grew one frame per
    // fact/form. A synthetic 5,000-fact deffacts reproduces the same
    // shape as the real external file that first hit this (a 128-fact
    // block in animal-external.clp) without depending on that fixture or
    // network access — mirrors how tests/stack_safety.rs regression-tests
    // lib/core.lisp's own tail-recursive list utilities on a 100,000-element
    // list.
    let mut facts = String::new();
    for i in 0..5000 {
        facts.push_str(&format!("(number {i})"));
    }
    let source =
        format!(r#"(clips-import (quote ((deffacts many (goal x)) (deffacts nums {facts}))))"#);
    let imported = eval_import(&source);
    assert!(
        imported.contains("(number 0)") && imported.contains("(number 4999)"),
        "expected every fact to survive the import, got a result of length {}",
        imported.len()
    );
}

#[test]
fn clips_defrule_with_an_exists_condition_imports_and_converts_its_template() {
    // Once lib/forward.my Step 15 gave `exists`/`forall` real dispatcher
    // support, the Step 19 skip-guard in this file was removed (same as
    // Step 5's `not` guard was removed after lib/forward.my Step 7).
    // `clips-convert-template` also learned to recurse into `exists`
    // (same lesson Step 13 taught for `not`/`or`/`and`), so a named-slot
    // fact nested inside it gets converted to positional form too.
    let source = r#"
        (clips-import (quote (
            (deftemplate unsolved (slot row) (slot column))
            (defrule wait-for-more
                (rank (value ?last))
                (exists (unsolved (row ?r) (column ?c)))
                =>
                (assert (rank (value done)))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((rank (value done)) (rank (value (var last))) (exists (unsolved (var r) (var c)))))"
    );
}

#[test]
fn clips_defrule_with_a_forall_condition_imports_correctly() {
    let source = r#"
        (clips-import (quote ((defrule all-checked
            (forall (item ?x) (checked ?x))
            =>
            (assert (done))))))
    "#;
    assert_eq!(
        eval_import(source),
        "(((done) (forall (item (var x)) (checked (var x)))))"
    );
}

fn eval_exact_clips_slot_lookup(source: &str) -> String {
    let mut session = Session::default();
    // Old eval_program(lib/core.lisp) interprets migrated binary heads as
    // numbers. This path uses the already proven, source-pinned D3/D4 loader.
    sens::eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("canonical source-pinned bootstrap");

    // Execute both original production definitions: slot-value-of + lookup.
    // Other historical CLIPS importer functions have separate migration
    // blockers and are not dependencies of this purely structural search.
    let full = include_str!("../../../lib/clips-import.lisp");
    let begin = full.find("(0011 clips-slot-value-of").unwrap();
    let end = full[begin..]
        .find("(0011 clips-positional-args")
        .map(|relative| begin + relative)
        .unwrap();
    let definitions = sens::parse_mixed_exact_domain(&full[begin..end])
        .expect("real D3/D4 CLIPS definition parses");
    sens::eval_parsed_expressions(&definitions, &mut session)
        .expect("real CLIPS slot definition loads");
    let request = sens::parse_mixed_exact_domain(source)
        .expect("D3 quoted test input parses");
    sens::eval_parsed_expressions(&request, &mut session)
        .expect("real D3 EQ slot lookup executes")
        .value
        .to_string()
}

#[test]
fn clips_slot_lookup_scans_named_atom_slots_without_d8_structural_alias() {
    // Regress against unreachable lookup; execute production D3/D4 definition.
    for (source, expected) in [
        ("(clips-lookup-slot-value (quote name) (quote ((name alice) (age 42))))", "alice"),
        ("(clips-lookup-slot-value (quote name) (quote ((age 42) (name bob))))", "bob"),
        ("(clips-lookup-slot-value (quote name) (quote ((age 42) (colour red))))", "()"),
        ("(clips-lookup-slot-value (quote name) (quote (bare-atom (name alice))))", "alice"),
        ("(clips-lookup-slot-value (quote name) (quote (((nested head) ignored) (name alice))))", "alice"),
    ] {
        assert_eq!(eval_exact_clips_slot_lookup(source), expected, "slot traversal failed for {source}");
    }
}

/// SENS decides the positive and negative D1 statements in a binary-headed
/// fixture. Rust owns only source-pinned definition loading and fail-closed
/// observation of the deliberately false one-bit expectation.
#[test]
fn clips_defrule_deftemplate_d1_self_verdict_and_mutant() {
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("test_id=clips-deftemplate-d1: pinned Core must load");
    let library = include_str!("../../../lib/clips-import.lisp");
    let start = library.find("(0011 clips-deftemplate-form?\n")
        .expect("test_id=clips-deftemplate-d1: real production definition exists");
    let end = library[start..].find("(0011 clips-string-empty?")
        .map(|offset| start + offset)
        .expect("test_id=clips-deftemplate-d1: next independent definition exists");
    let source = sens::parse_mixed_exact_domain(&library[start..end])
        .expect("test_id=clips-deftemplate-d1: exact production D3/D4 definitions parse");
    eval_parsed_expressions(&source, &mut session)
        .expect("test_id=clips-deftemplate-d1: production definition loads");

    let program = include_str!("../../../tests/semantic/clips-deftemplate-d1-law.lisp");
    let forms = sens::parse_mixed_exact_domain(program)
        .expect("test_id=clips-deftemplate-d1: SENS-owned binary law parses");
    assert_eq!(forms.len(), 3, "two SENS laws and one single-D1-bit mutant");
    for (index, form) in forms[..2].iter().enumerate() {
        let value = eval_parsed_expressions(std::slice::from_ref(form), &mut session)
            .unwrap_or_else(|error| panic!(
                "test_id=clips-deftemplate-d1 layer=eval law={index} error={error}"
            )).value;
        assert_eq!(
            value.as_predicate_bit(), Some(true),
            "test_id=clips-deftemplate-d1 layer=eval law={index} is not SENS D1:YES"
        );
    }
    let err = eval_parsed_expressions(&forms[2..], &mut session)
        .expect_err("test_id=clips-deftemplate-d1: one-bit mutant must fail closed");
    assert!(
        err.to_string().contains("domain identity is not callable under its ratified law: D1"),
        "test_id=clips-deftemplate-d1 layer=eval wrong failure class: {err}"
    );
}

#[test]
fn clips_defrule_form_dispatch_d1_self_verdict_and_one_bit_mutant() {
    // Rust transports real SENS source; D3 EQ and COND own all three YES
    // results and the one-bit NO mutant, not a host-side result table.
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    )
    .expect("test_id=clips-form-dispatch-d1: pinned Core must load");

    let source = include_str!("../../../lib/clips-import.lisp");
    let start = source.find("(0011 clips-form->clauses\n")
        .expect("test_id=clips-form-dispatch-d1: real dispatcher exists");
    let end = source[start..].find("(0011 clips-cons-each-onto\n")
        .map(|offset| start + offset)
        .expect("test_id=clips-form-dispatch-d1: next source definition exists");
    let functions = parse_mixed_exact_domain(&source[start..end])
        .expect("test_id=clips-form-dispatch-d1: source code parses");
    eval_parsed_expressions(&functions, &mut session)
        .expect("test_id=clips-form-dispatch-d1: source function loads");

    let fixture = include_str!("../../../tests/semantic/clips-form-dispatch-d1-law.lisp");
    let bodies: Vec<_> = fixture.split("(110\n").skip(1).collect();
    assert_eq!(bodies.len(), 4, "three SENS laws and one falsifier");
    assert_eq!(bodies[0].len(), bodies[3].len(), "mutant preserves D2 form");
    let substitutions: Vec<_> = bodies[0].bytes().zip(bodies[3].bytes())
        .filter(|(before, after)| before != after)
        .collect();
    assert_eq!(substitutions, vec![(b'1', b'0')],
        "the negative fixture must change precisely one D1 bit");

    let forms = parse_mixed_exact_domain(fixture)
        .expect("test_id=clips-form-dispatch-d1: all SENS laws must parse");
    assert_eq!(forms.len(), 4);
    for (index, form) in forms[..3].iter().enumerate() {
        let observed = eval_parsed_expressions(std::slice::from_ref(form), &mut session)
            .unwrap_or_else(|err| panic!(
                "test_id=clips-form-dispatch-d1 layer=eval law={index} failed: {err}"
            )).value;
        assert_eq!(observed.as_predicate_bit(), Some(true),
            "test_id=clips-form-dispatch-d1 layer=eval law={index} did not yield exact D1:YES");
    }
    let negative = eval_parsed_expressions(&forms[3..], &mut session)
        .expect("test_id=clips-form-dispatch-d1: one-bit mutant is a valid SENS D1:NO program")
        .value;
    assert_eq!(negative.as_predicate_bit(), Some(false),
        "test_id=clips-form-dispatch-d1 layer=eval one-D1-bit mutant must be exact D1:NO");
    println!(
        "SENS_CLIPS_DISPATCH_SELF_VERDICT id=clips-form-dispatch-d1 law=3 mutant=one-D1-bit outcome=PASS"
    );
}
