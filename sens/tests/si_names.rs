use sens::{eval_program, Session};

fn eval_with_si(source: &str) -> String {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/si.lisp"), &mut session)
        .expect("SI definitions should load");
    eval_program(source, &mut session)
        .expect("SI expression should evaluate")
        .value
        .to_string()
}

#[test]
fn descriptive_si_names_are_the_canonical_surface() {
    assert_eq!(eval_with_si("si:cesium-frequency"), "9192631770");
    assert_eq!(eval_with_si("si:speed-of-light"), "299792458");
    assert_eq!(eval_with_si("si:luminous-efficacy"), "683");
}

#[test]
fn descriptive_names_preserve_exact_values() {
    assert_eq!(
        eval_with_si("si:planck-constant"),
        "132521403/200000000000000000000000000000000000000000"
    );
    assert_eq!(
        eval_with_si("si:elementary-charge"),
        "801088317/5000000000000000000000000000"
    );
    assert_eq!(
        eval_with_si("si:boltzmann-constant"),
        "1380649/100000000000000000000000000000"
    );
    assert_eq!(
        eval_with_si("si:avogadro-constant"),
        "602214076000000000000000"
    );
}
