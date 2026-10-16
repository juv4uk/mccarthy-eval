use std::fs;
use std::path::PathBuf;

use sens::semantic_registry_export::semantic_id_for_admitted_surface;
use sens::{eval_program, load_core_library, lower_program, parse, ExprKind, Session};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf()
}

fn session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must load");
    let source = fs::read_to_string(repo_root().join("lib/machine/capability-axis.lisp"))
        .expect("capability axis source");
    eval_program(&source, &mut session).expect("capability axis must load");
    session
}

#[test]
fn current_exact_domain_identities_project_to_target_neutral_capabilities() {
    let lowered = lower_program(&parse("(додати 2 3)").expect("PLUS source"));
    let ExprKind::DomainCall(add_identity, _) = lowered[0].kind else {
        panic!("current PLUS surface must lower to DomainCall");
    };
    assert_eq!((add_identity.width(), add_identity.packed_bits()), (5, 0b01010));

    let lowered = lower_program(&parse("(перше '(a b))").expect("CAR source"));
    let ExprKind::DomainCall(car_identity, _) = lowered[0].kind else {
        panic!("current CAR surface must lower to DomainCall");
    };
    assert_eq!((car_identity.width(), car_identity.packed_bits()), (3, 0b100));

    let mut s = session();
    let add = eval_program(
        &format!(
            "(machine-capabilities-for-domain {} {})",
            add_identity.width(),
            add_identity.packed_bits()
        ),
        &mut s,
    )
    .expect("D5 PLUS capability lookup")
    .value
    .to_string();
    assert_eq!(add, "((integer-add bounded-u32-inputs u64-result))");

    let car = eval_program(
        &format!(
            "(machine-capabilities-for-domain {} {})",
            car_identity.width(),
            car_identity.packed_bits()
        ),
        &mut s,
    )
    .expect("D3 CAR capability lookup")
    .value
    .to_string();
    assert_eq!(car, "((pair-field-load head bounded-u64))");

    let quoted_shadow = eval_program(
        r#"(machine-capabilities-for-domain 5 "01010")"#,
        &mut s,
    )
    .expect("quoted domain shadow must remain ordinary String data")
    .value
    .to_string();
    assert_eq!(
        quoted_shadow, "()",
        "String data must not act as exact-domain machine-capability identity"
    );
}

#[test]
fn packed_bits_do_not_cross_domain_widths() {
    let mut s = session();

    assert_eq!(
        eval_program("(machine-capabilities-for-domain 5 10)", &mut s)
            .expect("D5:01010 PLUS")
            .value
            .to_string(),
        "((integer-add bounded-u32-inputs u64-result))"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 4 10)", &mut s)
            .expect("D4:1010 LOOKUP must not inherit PLUS")
            .value
            .to_string(),
        "()"
    );

    assert_eq!(
        eval_program("(machine-capabilities-for-domain 5 11)", &mut s)
            .expect("D5:01011 DIFFERENCE")
            .value
            .to_string(),
        "((integer-subtract bounded-u64 no-underflow))"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 4 11)", &mut s)
            .expect("D4:1011 BIND must not inherit DIFFERENCE")
            .value
            .to_string(),
        "()"
    );

    assert_eq!(
        eval_program("(machine-capabilities-for-domain 5 8)", &mut s)
            .expect("D5:01000 ZEROP")
            .value
            .to_string(),
        "((integer-zero-test bounded-nonnegative-u64 internal-bit-d1-boundary))"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 4 8)", &mut s)
            .expect("D4:1000 CAAR must not inherit ZEROP")
            .value
            .to_string(),
        "()"
    );


    assert_eq!(
        eval_program("(machine-capabilities-for-domain 6 14)", &mut s)
            .expect("D6:001110 ADD1")
            .value
            .to_string(),
        "((integer-increment bounded-u64 no-overflow))"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 5 14)", &mut s)
            .expect("D5:01110 selector must not inherit D6 ADD1")
            .value
            .to_string(),
        "()"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 6 15)", &mut s)
            .expect("D6:001111 SUB1")
            .value
            .to_string(),
        "((integer-decrement bounded-u64 no-underflow))"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 5 15)", &mut s)
            .expect("D5:01111 selector must not inherit D6 SUB1")
            .value
            .to_string(),
        "()"
    );
}

#[test]
fn active_machine_capability_axis_has_no_sid8_lookup() {
    let source = fs::read_to_string(repo_root().join("lib/machine/capability-axis.lisp"))
        .expect("capability axis source");

    assert!(source.contains("machine-capability-axis-v3"));
    assert!(source.contains("machine-capabilities-for-domain"));
    assert!(!source.contains("machine-capability-legacy-sid-axis"));
    assert!(!source.contains("machine-capabilities-for-sid"));
    assert!(!source.contains("00001100"));
}

#[test]
fn capability_names_do_not_mint_semantic_identities() {
    for capability in [
        "integer-zero-test",
        "integer-add",
        "integer-subtract",
        "integer-multiply",
        "integer-quotient",
        "integer-order-less",
        "integer-order-greater",
        "integer-increment",
        "integer-decrement",
        "identity-compare",
        "conditional-branch",
        "pair-field-store",
        "pair-field-load",
        "bounded-u64",
    ] {
        assert_eq!(
            semantic_id_for_admitted_surface(capability),
            None,
            "{capability} is machine-description data, not a language identity"
        );
    }
}

#[test]
fn unsupported_targets_are_explicitly_unwitnessed() {
    let mut s = session();

    for target in ["arm64", "risc-v", "fpga"] {
        let expression = format!("(machine-target-witness-status (quote {target}))");
        let value = eval_program(&expression, &mut s)
            .expect("target status lookup")
            .value
            .to_string();
        assert_eq!(value, "absent");
    }

    let x86 = eval_program(
        "(machine-target-witness-status (quote x86-64))",
        &mut s,
    )
    .expect("x86 witness status")
    .value
    .to_string();
    assert_eq!(x86, "witnessed");
}

#[test]
fn x86_target_witnesses_reference_existing_lowering_names_without_owning_meaning() {
    let source = fs::read_to_string(repo_root().join("lib/machine/capability-axis.lisp"))
        .expect("capability axis");
    let lowering = fs::read_to_string(repo_root().join("lib/machine/lowering/semantic-x86-64.lisp"))
        .expect("x86 semantic lowering");

    for name in [
        "x86-lower-zerop-u64-forms",
        "x86-lower-add-u64-forms",
        "x86-lower-difference-u64-forms",
        "x86-lower-times-u64-forms",
        "x86-lower-quotient-i64-equal-forms",
        "x86-lower-order-i64-forms",
        "x86-lower-eq-cond-u64-forms",
        "x86-lower-bounded-pair-store-u64-forms",
        "x86-lower-cons-car-u64-forms",
        "x86-lower-cons-cdr-u64-forms",
    ] {
        assert!(source.contains(name), "axis references {name}");
        assert!(
            lowering.contains(&format!("(00001001 {name}")),
            "referenced x86 witness {name} must exist"
        );
    }
}
