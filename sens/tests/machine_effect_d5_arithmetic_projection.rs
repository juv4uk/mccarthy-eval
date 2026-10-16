use sens::{
    eval_parsed_expressions, eval_program, load_core_library, lower_program,
    parse, parse_mixed_exact_domain_machine_source, ExprKind, Session,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[path = "support/x86_64_block_decoder.rs"]
mod x86_64_block_decoder;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|error| panic!("{path} must exist: {error}"))
}

fn load_lisp_file(path: &str, session: &mut Session) {
    assert!(
        path.starts_with("lib/machine/"),
        "machine-source reader is restricted to lib/machine/** fixtures: {path}"
    );
    let source = read(path);
    let expressions = parse_mixed_exact_domain_machine_source(path, &source)
        .unwrap_or_else(|error| panic!("{path} must parse as exact-domain machine source: {error}"));
    eval_parsed_expressions(&expressions, session)
        .unwrap_or_else(|error| panic!("{path} must load through the machine-source reader: {error}"));
}

fn eval_value(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .value
        .to_string()
}

fn parse_bytes(rendered: &str) -> Vec<u8> {
    rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| token.parse::<u8>().expect("machine byte"))
        .collect()
}

fn render_decoded(forms: &[String]) -> String {
    format!("({})", forms.join(" "))
}

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core");
    load_lisp_file("lib/machine/effects/u64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-effects.lisp", &mut session);
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/operands/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/atoms/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/projection/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-x86-64.lisp", &mut session);
    session
}

fn current_d5_coordinate(resident: &str) -> (usize, u8) {
    let foundation: Value =
        serde_json::from_str(&read("knowledge/d1-d9-foundation.json")).expect("foundation JSON");
    let d5 = &foundation["domains"]["D5"];
    let width = d5["width"].as_u64().expect("D5 width") as usize;
    let residents = d5["residents"].as_object().expect("D5 residents");
    let (bits, _) = residents
        .iter()
        .find(|(_, role)| role.as_str() == Some(resident))
        .unwrap_or_else(|| panic!("current D5 foundation must contain {resident}"));
    (
        width,
        u8::from_str_radix(bits, 2).expect("current D5 resident bits"),
    )
}

fn lowered_identity(source: &str) -> (usize, u8) {
    let parsed = parse(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    let lowered = lower_program(&parsed);
    let ExprKind::DomainCall(identity, _) = &lowered[0].kind else {
        panic!("{source} must lower to exact DomainCall");
    };
    (identity.width(), identity.packed_bits())
}

#[test]
fn exact_d5_identity_selects_target_neutral_add_sub_mul_effects() {
    let cases = [
        ("PLUS", "(додати 2 3)", 2u64, 3u64, "(bounded-u64-add 2 3)"),
        (
            "DIFFERENCE",
            "(відняти 5 3)",
            5u64,
            3u64,
            "(bounded-u64-sub 5 3)",
        ),
        (
            "TIMES",
            "(помножити 4 6)",
            4u64,
            6u64,
            "(bounded-u64-mul 4 6)",
        ),
    ];

    let effect_source = read("lib/machine/effects/u64.lisp").to_ascii_lowercase();
    for forbidden in [
        "x86",
        "rax",
        "rcx",
        "mov-r64",
        "sub-r64",
        "imul-r64",
        "modrm",
        "vex",
        "machine-effect-current-domain-key?",
        "machine-lower-current-binary-effect",
    ] {
        assert!(
            !effect_source.contains(forbidden),
            "target-neutral effect library leaked authority/target token {forbidden}"
        );
    }

    let mut session = machine_session();
    for (resident, source, left, right, expected_effect) in cases {
        let expected_identity = current_d5_coordinate(resident);
        let identity = lowered_identity(source);
        assert_eq!(identity, expected_identity, "{source}");

        let selected = eval_value(
            &format!(
                "(machine-lower-current-binary-effect {} {} {left} {right})",
                identity.0, identity.1
            ),
            &mut session,
        );
        assert_eq!(selected, expected_effect, "{source}");
    }

    let quotient = current_d5_coordinate("QUOTIENT");
    assert_eq!(
        eval_value(
            &format!(
                "(machine-lower-current-binary-effect {} {} 4 2)",
                quotient.0, quotient.1
            ),
            &mut session,
        ),
        "machine-effect-not-applicable",
        "QUOTIENT belongs to its separate effect/lowering slice"
    );
}

#[test]
fn bounded_sub_effect_projects_with_form_byte_and_independent_decode_parity() {
    let mut session = machine_session();

    for (left, right) in [(5u64, 3u64), (u64::MAX, 0), (u64::MAX, u64::MAX)] {
        let effect = eval_value(
            &format!("(machine-effect-bounded-u64-sub {left} {right})"),
            &mut session,
        );
        assert_eq!(effect, format!("(bounded-u64-sub {left} {right})"));

        let projected = eval_value(
            &format!("(x86-project-machine-effect (quote {effect}))"),
            &mut session,
        );
        let donor = eval_value(
            &format!("(x86-lower-difference-u64-forms {left} {right})"),
            &mut session,
        );
        assert_eq!(projected, donor, "SUB projection must preserve donor forms");

        let projected_bytes = eval_value(
            &format!("(x86-encode-machine-effect (quote {effect}))"),
            &mut session,
        );
        let donor_bytes = eval_value(
            &format!("(x86-encode-admitted-program (quote {donor}))"),
            &mut session,
        );
        assert_eq!(projected_bytes, donor_bytes, "SUB bytes must match donor");

        let bytes = parse_bytes(&projected_bytes);
        let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
            .unwrap_or_else(|error| panic!("independent decoder rejected {bytes:?}: {error}"));
        assert_eq!(
            render_decoded(&decoded),
            projected,
            "existing observer must reconstruct SUB projected forms"
        );
    }
}

#[test]
fn bounded_mul_effect_projects_with_form_and_byte_parity() {
    let mut session = machine_session();

    for (left, right) in [
        (2u64, 3u64),
        (0, u32::MAX as u64),
        (u32::MAX as u64, u32::MAX as u64),
    ] {
        let effect = eval_value(
            &format!("(machine-effect-bounded-u64-mul {left} {right})"),
            &mut session,
        );
        assert_eq!(effect, format!("(bounded-u64-mul {left} {right})"));

        let projected = eval_value(
            &format!("(x86-project-machine-effect (quote {effect}))"),
            &mut session,
        );
        let donor = eval_value(
            &format!("(x86-lower-times-u64-forms {left} {right})"),
            &mut session,
        );
        assert_eq!(projected, donor, "IMUL projection must preserve donor forms");

        let projected_bytes = eval_value(
            &format!("(x86-encode-machine-effect (quote {effect}))"),
            &mut session,
        );
        let donor_bytes = eval_value(
            &format!("(x86-encode-admitted-program (quote {donor}))"),
            &mut session,
        );
        assert_eq!(projected_bytes, donor_bytes, "IMUL bytes must match donor");
    }

    // Independent IMUL decode expansion stays in #4343 ownership.
}

#[test]
fn sub_mul_effect_bounds_fail_closed_before_target_projection() {
    let mut session = machine_session();

    for source in [
        "(machine-effect-bounded-u64-sub 2 3)",
        "(machine-effect-bounded-u64-sub -1 0)",
        "(machine-effect-bounded-u64-sub 18446744073709551616 0)",
        "(machine-effect-bounded-u64-mul 4294967296 1)",
        "(machine-effect-bounded-u64-mul 1 4294967296)",
    ] {
        assert_eq!(
            eval_value(source, &mut session),
            "machine-effect-rejected",
            "{source}"
        );
    }

    let difference = current_d5_coordinate("DIFFERENCE");
    assert_eq!(
        eval_value(
            &format!(
                "(machine-lower-current-binary-effect {} {} 2 3)",
                difference.0, difference.1
            ),
            &mut session,
        ),
        "machine-effect-rejected",
        "valid DIFFERENCE identity with unsafe bounded carrier must reject as an effect"
    );
}

#[test]
fn bounded_effect_source_keeps_exact_d1_controls() {
    // #5360: застаріла масова міграція не має повертати числове 1 чи історичне t у D3 COND.
    let text = read("lib/machine/effects/u64.lisp");
    for (number, line) in text.lines().enumerate() {
        let active = line.split(';').next().unwrap_or("").trim_start();
        assert!(
            !active.starts_with("(1 ")
                && !active.starts_with("(1\t")
                && !active.contains(" t)"),
            "Джерело D5 повернуло неточний D1 у рядку {}: {}",
            number + 1,
            line
        );
    }
    assert!(
        text.matches("(00000010 (00000001 ()))").count() >= 8,
        "Вісім доведених предикатних відповідей D1:YES мають залишатися в джерелі"
    );
    for застарілий_код in ["(00000011 ", "(00100010 "] {
        assert!(
            !text.contains(застарілий_код),
            "Історичний 8-бітовий виклик рівності знову з'явився в D5: {застарілий_код}"
        );
    }
}

#[test]
fn bounded_effect_negative_form_predicates_return_exact_d1_no() {
    // #5361: D1:0 не є порожнім списком; у COND дозволений лише точний PredicateBit.
    let mut session = Session::default();
    load_core_library(&mut session).expect("ядро");
    load_lisp_file("lib/machine/effects/u64.lisp", &mut session);

    for source in [
        "(machine-effect-bounded-u64-add-form? (00000001 ()))",
        "(machine-effect-bounded-u64-sub-form? (00000001 ()))",
        "(machine-effect-bounded-u64-mul-form? (00000001 ()))",
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"))
            .value;
        assert_eq!(
            actual.as_predicate_bit(),
            Some(false),
            "{source} має повернути точний D1:0, а не () чи числовий нуль"
        );
    }
}

#[test]
fn bounded_effect_integer_range_answer_is_exact_d1() {
    // #5361: негативний вихід за будь-яку межу має бути D1:0, а не ().
    let mut session = Session::default();
    load_core_library(&mut session).expect("ядро");
    load_lisp_file("lib/machine/effects/u64.lisp", &mut session);
    for (source, expected) in [
        ("(machine-effect-within-inclusive-integer-range? 5 0 10)", true),
        ("(machine-effect-within-inclusive-integer-range? -1 0 10)", false),
        ("(machine-effect-within-inclusive-integer-range? 11 0 10)", false),
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"))
            .value;
        assert_eq!(
            actual.as_predicate_bit(),
            Some(expected),
            "{source}: результат має належати точному домену D1"
        );
    }
}

#[test]
fn bounded_effect_carriers_reject_out_of_range_with_exact_d1_no() {
    // #5361: структурне () не може керувати строгим D3 COND.
    let mut session = Session::default();
    load_core_library(&mut session).expect("ядро");
    load_lisp_file("lib/machine/effects/u64.lisp", &mut session);
    for source in [
        "(machine-effect-u32-carrier? 4294967296)",
        "(machine-effect-u64-carrier? -1)",
        "(machine-effect-u64-carrier? 18446744073709551616)",
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"))
            .value;
        assert_eq!(
            actual.as_predicate_bit(),
            Some(false),
            "{source}: позадоменний носій мусить повернути точний D1:0"
        );
    }
}

#[test]
fn bounded_effect_structural_forms_return_exact_d1_for_matching_and_mismatched_kinds() {
    // #5354: позитивні і чужі ефекти мають давати типізований D1, а не Lisp truthiness.
    let mut session = Session::default();
    load_core_library(&mut session).expect("ядро");
    load_lisp_file("lib/machine/effects/u64.lisp", &mut session);
    for (source, expected) in [
        ("(machine-effect-bounded-u64-add-form? (quote (bounded-u64-add 2 3)))", true),
        ("(machine-effect-bounded-u64-sub-form? (quote (bounded-u64-sub 5 3)))", true),
        ("(machine-effect-bounded-u64-mul-form? (quote (bounded-u64-mul 2 3)))", true),
        ("(machine-effect-bounded-u64-add-form? (quote (bounded-u64-sub 5 3)))", false),
        ("(machine-effect-bounded-u64-sub-form? (quote (bounded-u64-mul 2 3)))", false),
        ("(machine-effect-bounded-u64-mul-form? (quote (bounded-u64-add 2 3)))", false),
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"))
            .value;
        assert_eq!(
            actual.as_predicate_bit(),
            Some(expected),
            "{source}: відповідь мусить бути точним D1-предикатом"
        );
    }
}


/// Окремий falsifier: ратифіковані D5/D6 порівняння повинні
/// відрізняти арифметичний носій від точного предиката D1.
/// Усі кейси виконуються до фінальної відмови, щоб first-fail не приховав решту.
#[test]
fn d5_d6_comparison_truth_table_is_exact_d1() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("канонічне ядро");
    let witnesses = [
        ("(не-менше? -1 0)", false),
        ("(не-менше? 0 0)", true),
        ("(не-менше? 1 0)", true),
        ("(не-менше? 0 1)", false),
        ("(не-більше? -1 0)", true),
        ("(не-більше? 0 0)", true),
        ("(не-більше? 1 0)", false),
        ("(не-більше? 0 1)", true),
        ("(менше? 2 3)", true),
        ("(менше? 3 2)", false),
        ("(менше? -1 0)", true),
        ("(менше? 0 -1)", false),
        ("(менше? 4294967295 4294967296)", true),
        ("(менше? 4294967296 4294967295)", false),
        ("(не-більше? 4294967296 4294967295)", false),
        ("(не-менше? 4294967296 0)", true),
    ];
    let mut violations = Vec::new();
    for (source, expected) in witnesses {
        match eval_program(source, &mut session) {
            Ok(answer) => {
                let actual = answer.value.as_predicate_bit();
                eprintln!(
                    "D6_ТОЧНИЙ_СВІДОК expression={source} rendered={} d1={actual:?} expected={expected}",
                    answer.value
                );
                if actual != Some(expected) {
                    violations.push(format!(
                        "{source}: d1={actual:?}, value={}, expected={expected}",
                        answer.value
                    ));
                }
            }
            Err(error) => {
                eprintln!("D6_НЕВИКЛИЧНИЙ_СВІДОК expression={source} error={error}");
                violations.push(format!("{source}: {error}"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Порушено точні D1/D6 межі (негативи не можна вважати PASS): {}",
        violations.join("; ")
    );
}
