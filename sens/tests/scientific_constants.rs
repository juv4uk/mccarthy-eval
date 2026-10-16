use sens::{eval_program, Session};

fn eval_science(source: &str) -> String {
    let mut session = Session::default();
    for library in [
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/quantity.lisp"),
        include_str!("../../../lib/si.lisp"),
    ] {
        eval_program(library, &mut session).expect("science library should load");
    }
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("science expression failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

fn eval_science_knowledge(source: &str) -> String {
    let mut session = Session::default();
    for library in [
        include_str!("../../../lib/core.lisp"),
        include_str!("../../../lib/unify.lisp"),
        include_str!("../../../lib/reason.lisp"),
        include_str!("../../../lib/forward.lisp"),
        include_str!("../../../lib/knowledge.lisp"),
        include_str!("../../../lib/result-status.lisp"),
        include_str!("../../../lib/quantity.lisp"),
        include_str!("../../../lib/si.lisp"),
    ] {
        eval_program(library, &mut session).expect("science/knowledge library should load");
    }
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("science knowledge expression failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn speed_of_light_record_keeps_value_unit_status_kind_system_and_source() {
    let source = r#"
        (list
          (scientific-constant-name si:defining-speed-of-light)
          (scientific-constant-value si:defining-speed-of-light)
          (unit-dimensions
            (scientific-constant-unit si:defining-speed-of-light))
          (scientific-constant-status si:defining-speed-of-light)
          (scientific-constant-kind si:defining-speed-of-light)
          (scientific-constant-system si:defining-speed-of-light)
          (scientific-constant-source si:defining-speed-of-light))
    "#;
    assert_eq!(
        eval_science(source),
        "(si:speed-of-light 299792458 ((dimension/1 metre 1) (dimension/1 second -1)) exact-by-definition physical-defining si (science-source/1 bipm-si-brochure-9 2019))"
    );
}

#[test]
fn exact_rational_values_remain_unchanged_after_structuring() {
    assert_eq!(
        eval_science("si:planck-constant"),
        "132521403/200000000000000000000000000000000000000000"
    );
    assert_eq!(
        eval_science("si:elementary-charge"),
        "801088317/5000000000000000000000000000"
    );
    assert_eq!(
        eval_science("si:boltzmann-constant"),
        "1380649/100000000000000000000000000000"
    );
    assert_eq!(
        eval_science("si:avogadro-constant"),
        "602214076000000000000000"
    );
}

#[test]
fn advice_taker_can_reason_about_an_admitted_scientific_constant() {
    let source = r#"
        (def expected-unit
          (quote
            (unit/1
              (dimension/1 metre 1)
              (dimension/1 second -1))))
        (def admission
          (advise-all science
            (scientific-constant->clauses si:defining-speed-of-light)))
        (list
          (car admission)
          (result-status
            (reason-in-observe
              (quote science)
              (quote (constant-value si:speed-of-light 299792458))))
          (result-status
            (reason-in-observe
              (quote science)
              (quote (constant-status si:speed-of-light exact-by-definition))))
          (result-status
            (reason-in-observe
              (quote science)
              (list
                (quote constant-unit)
                (quote si:speed-of-light)
                expected-unit))))
    "#;
    assert_eq!(eval_science_knowledge(source), "(accepted proved proved proved)");
}

#[test]
fn malformed_constant_cannot_project_into_knowledge() {
    let source = r#"
        (scientific-constant->clauses
          (quote (scientific-constant/1 broken)))
    "#;
    assert_eq!(eval_science(source), "()");
}

/// `si` defines `rest` (`"name value)"`) — by SENS code 00001001 or the old
/// `def` name.
fn si_defines(si: &str, rest: &str) -> bool {
    ["(00001001 ", "(def "]
        .iter()
        .any(|head| si.contains(&format!("{head}{rest}")))
}

#[test]
fn si_source_does_not_reintroduce_a_second_literal_authority_for_numeric_views() {
    let si = include_str!("../../../lib/si.lisp");

    for forbidden in [
        "si:cesium-frequency 9192631770)",
        "si:speed-of-light 299792458)",
        "si:avogadro-constant 602214076000000000000000)",
        "si:luminous-efficacy 683)",
    ] {
        assert!(
            !si_defines(si, forbidden),
            "numeric view must be derived from its scientific-constant record: {forbidden}"
        );
    }

    for derived in [
        "si:cesium-frequency (si:constant-value si:defining-cesium-frequency))",
        "si:speed-of-light (si:constant-value si:defining-speed-of-light))",
        "si:planck-constant (si:constant-value si:defining-planck-constant))",
        "si:elementary-charge (si:constant-value si:defining-elementary-charge))",
        "si:boltzmann-constant (si:constant-value si:defining-boltzmann-constant))",
        "si:avogadro-constant (si:constant-value si:defining-avogadro-constant))",
        "si:luminous-efficacy (si:constant-value si:defining-luminous-efficacy))",
    ] {
        assert!(si_defines(si, derived), "missing derived SI numeric view: {derived}");
    }
}
