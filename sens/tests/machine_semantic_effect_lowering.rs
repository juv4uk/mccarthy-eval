use sens::{eval_program, load_core_library, lower_program, parse, ExprKind, Session};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|error| panic!("#4365 requires {path}: {error}"))
}

fn load_lisp_file(path: &str, session: &mut Session) {
    let source = read(path);
    eval_program(&source, session)
        .unwrap_or_else(|error| panic!("{path} must load as ordinary sens: {error}"));
}

fn eval_value(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .value
        .to_string()
}

fn lowering_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core");
    load_lisp_file("lib/machine/effects/u64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-effects.lisp", &mut session);
    session
}

fn current_d5_coordinate(resident: &str) -> (usize, u8) {
    let foundation: Value =
        serde_json::from_str(&read("knowledge/d1-d9-foundation.json")).expect("foundation JSON");

    assert_eq!(
        foundation["schema"], "d1-d9-foundation-ratification/v1",
        "#4365 must consume the current foundation schema"
    );
    assert_eq!(
        foundation["status"], "owner-ratified",
        "#4365 must not route from a research/shadow map"
    );

    let d5 = &foundation["domains"]["D5"];
    let width = d5["width"].as_u64().expect("D5 width") as usize;
    let residents = d5["residents"].as_object().expect("D5 residents");
    let (bits, _) = residents
        .iter()
        .find(|(_, role)| role.as_str() == Some(resident))
        .unwrap_or_else(|| panic!("current D5 foundation must contain {resident}"));

    (
        width,
        u8::from_str_radix(bits, 2).expect("current D5 PLUS bits"),
    )
}

#[test]
fn current_foundation_runtime_identity_and_effect_router_agree() {
    let cases = [
        ("PLUS", "(додати 2 3)"),
        ("DIFFERENCE", "(відняти 5 3)"),
        ("TIMES", "(помножити 4 6)"),
    ];

    let source = read("lib/machine/lowering/semantic-effects.lisp");
    for (resident, program) in cases {
        let expected = current_d5_coordinate(resident);

        let parsed = parse(program).expect("current D5 source");
        let lowered = lower_program(&parsed);
        let ExprKind::DomainCall(identity, args) = &lowered[0].kind else {
            panic!("{resident} must lower to exact DomainCall");
        };

        assert_eq!(
            (identity.width(), identity.packed_bits()),
            expected,
            "runtime exact DomainIdentity and owner foundation must agree for {resident}"
        );
        assert_eq!(args.len(), 2);

        let guarded_coordinate = format!(
            "(machine-effect-current-domain-key? width bits {} {})",
            expected.0, expected.1
        );
        assert!(
            source.contains(&guarded_coordinate),
            "semantic effect router drifted from owner foundation for {resident}: expected {guarded_coordinate}"
        );
    }
}

#[test]
fn exact_d5_arithmetic_selects_only_admitted_target_neutral_effects() {
    let mut session = lowering_session();
    let cases = [
        ("PLUS", 2u64, 3u64, "(bounded-u64-add 2 3)"),
        ("DIFFERENCE", 5, 3, "(bounded-u64-sub 5 3)"),
        ("TIMES", 4, 6, "(bounded-u64-mul 4 6)"),
    ];

    for (resident, left, right, expected_effect) in cases {
        let (width, bits) = current_d5_coordinate(resident);
        assert_eq!(
            eval_value(
                &format!("(machine-lower-current-binary-effect {width} {bits} {left} {right})"),
                &mut session,
            ),
            expected_effect,
            "{resident}"
        );
    }

    let (plus_width, plus_bits) = current_d5_coordinate("PLUS");
    assert_eq!(
        eval_value(
            &format!(
                "(machine-lower-current-binary-effect {plus_width} {plus_bits} 4294967296 3)"
            ),
            &mut session,
        ),
        "machine-effect-rejected",
        "PLUS must preserve the bounded u32 carrier guard"
    );

    let (difference_width, difference_bits) = current_d5_coordinate("DIFFERENCE");
    assert_eq!(
        eval_value(
            &format!(
                "(machine-lower-current-binary-effect {difference_width} {difference_bits} 2 3)"
            ),
            &mut session,
        ),
        "machine-effect-rejected",
        "DIFFERENCE underflow must fail closed before target projection"
    );

    let (times_width, times_bits) = current_d5_coordinate("TIMES");
    assert_eq!(
        eval_value(
            &format!(
                "(machine-lower-current-binary-effect {times_width} {times_bits} 4294967296 1)"
            ),
            &mut session,
        ),
        "machine-effect-rejected",
        "TIMES outside the proved u32×u32 rectangle must fail closed"
    );
}

#[test]
fn wrong_domain_and_unmapped_current_identity_fail_closed() {
    let (_, plus_bits) = current_d5_coordinate("PLUS");
    let mut session = lowering_session();

    for source in [
        format!("(machine-lower-current-binary-effect 4 {plus_bits} 2 3)"),
        format!("(machine-lower-current-binary-effect 6 {plus_bits} 2 3)"),
        "(machine-lower-current-binary-effect 5 23 4 2)".to_string(),
    ] {
        assert_eq!(
            eval_value(&source, &mut session),
            "machine-effect-not-applicable",
            "{source}"
        );
    }
}

#[test]
fn semantic_to_effect_layer_contains_no_target_or_surface_authority() {
    let lowerer = read("lib/machine/lowering/semantic-effects.lisp").to_ascii_lowercase();
    let effects = read("lib/machine/effects/u64.lisp").to_ascii_lowercase();

    for forbidden in [
        "x86",
        "rax",
        "rcx",
        "modrm",
        "vex",
        "avx",
        "cuda",
        "ptx",
        "fpga",
        "pair-x86",
        "intel",
        "semantic-registry",
        "додати",
    ] {
        assert!(
            !lowerer.contains(forbidden),
            "#4365 semantic->effect seam leaked target/surface authority token {forbidden}"
        );
    }

    assert!(
        !effects.contains("machine-lower-current-binary-effect")
            && !effects.contains("machine-effect-current-domain-key?"),
        "generic effect-definition module must not own semantic coordinate routing"
    );
}

#[test]
fn current_machine_effect_key_has_exact_d1_predicate_results() {
    // #5354: однаковий packed payload у різних доменах не змінює біт D1.
    let mut session = Session::default();
    load_core_library(&mut session).expect("ядро");
    load_lisp_file("lib/machine/lowering/semantic-effects.lisp", &mut session);
    for (source, expected) in [
        ("(machine-effect-current-domain-key? 5 10 5 10)", true),
        ("(machine-effect-current-domain-key? 4 10 5 10)", false),
        ("(machine-effect-current-domain-key? 5 11 5 10)", false),
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"))
            .value;
        assert_eq!(
            actual.as_predicate_bit(),
            Some(expected),
            "{source} має повернути точний D1-предикат, а не число чи порожню структуру"
        );
    }
}
