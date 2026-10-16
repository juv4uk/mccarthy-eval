use sens::{eval_program, eval_parsed_expressions, parse_mixed_exact_domain_machine_source, load_core_library, Session};
use std::fs;
use std::path::PathBuf;

#[path = "support/x86_64_block_decoder.rs"]
mod x86_64_block_decoder;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_lisp_file(path: &str, session: &mut Session) {
    let path = repo_root().join(path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
    let relative = path.strip_prefix(repo_root())
        .expect("Lisp fixture path must be inside the SENS repository");
    let source_path = relative.to_str()
        .expect("machine fixture path must be valid UTF-8");
    if source_path.starts_with("lib/machine/") {
        let parsed = parse_mixed_exact_domain_machine_source(source_path, &source)
            .unwrap_or_else(|error| panic!("{source_path}: exact-domain read failed: {error}"));
        eval_parsed_expressions(&parsed, session)
            .unwrap_or_else(|error| panic!("{source_path}: exact-domain execution failed: {error}"));
    } else {
        eval_program(&source, session)
            .unwrap_or_else(|error| panic!("{} must load as ordinary sens: {error}", path.display()));
    }
}

fn profile_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before #196 profile witness");
    load_lisp_file("lib/machine/block.lisp", &mut session);
    load_lisp_file("lib/machine/layout/pair-x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/operands/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/atoms/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/profile/minimal-runtime-x86-64.lisp", &mut session);
    session
}

fn eval_value(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("#196 profile expression failed: {source}: {error}"))
        .value
        .to_string()
}

fn parse_byte_list(rendered: &str) -> Vec<u8> {
    rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .map(|token| token.parse().expect("encoded machine byte must fit u8"))
        .collect()
}

fn render_forms(forms: &[String]) -> String {
    format!("({})", forms.join(" "))
}

#[test]
fn structural_car_profile_is_projected_from_the_existing_semantic_witness() {
    let mut session = profile_session();

    let semantic_forms = eval_value("(x86-lower-cons-car-u64-forms 2 3)", &mut session);
    let profile_forms = eval_value("(x86-minimal-structural-car-forms 2 3)", &mut session);
    assert_eq!(profile_forms, semantic_forms);

    let families = eval_value(
        "(x86-minimal-structural-car-observed-families 2 3)",
        &mut session,
    );
    assert_eq!(
        families,
        "(mov-r64-imm64 mov-mem-disp8-r64 mov-r64-mem-disp8 ret)"
    );

    let dependency_families = eval_value(
        "(x86-minimal-structural-car-dependency-families)",
        &mut session,
    );
    assert_eq!(dependency_families, families);

    let classes = eval_value(
        "(x86-minimal-structural-car-dependency-classes)",
        &mut session,
    );
    assert_eq!(
        classes,
        "(required-for-minimal-runtime required-for-minimal-runtime required-for-minimal-runtime required-for-minimal-runtime)"
    );
}

#[test]
fn structural_car_profile_has_typed_177_atom_projections_for_every_required_family() {
    let mut session = profile_session();

    for (expression, expected) in [
        (
            "(x86-mov-r64-imm64 (quote rax) 2)",
            "(mov-r64-imm64 rax 2)",
        ),
        (
            "(x86-mov-mem64-r64 (x86-mem64-disp8 (quote rdi) x86-pair-car-offset) (quote rax))",
            "(mov-mem-disp8-r64 rdi 0 rax)",
        ),
        (
            "(x86-mov-r64-mem64 (quote rax) (x86-mem64-disp8 (quote rdi) x86-pair-car-offset))",
            "(mov-r64-mem-disp8 rax rdi 0)",
        ),
        ("(x86-ret)", "(ret)"),
    ] {
        assert_eq!(eval_value(expression, &mut session), expected);
    }
}

#[test]
fn structural_car_profile_reaches_bytes_only_through_closed_admission() {
    let mut session = profile_session();
    let forms = eval_value("(x86-minimal-structural-car-forms 2 3)", &mut session);
    let encoded = eval_value(
        "(x86-encode-admitted-program-or-reject (x86-minimal-structural-car-forms 2 3))",
        &mut session,
    );

    assert!(encoded.starts_with('('), "expected encoded byte list, got {encoded}");
    assert!(
        !encoded.contains("rejected"),
        "minimal profile must not bypass or fail current admission: {encoded}"
    );

    let bytes = parse_byte_list(&encoded);
    let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
        .unwrap_or_else(|error| panic!("independent decoder rejected #196 witness {bytes:?}: {error}"));
    assert_eq!(render_forms(&decoded), forms);
}

#[test]
fn bounded_cond_profile_matches_lisp_owned_machine_lowering() {
    let mut session = profile_session();

    // Language-level EQ/COND meaning is owned by the current SENS/Lisp
    // contracts and witnesses, not by this Rust machine-profile test.
    // This test starts at the already-selected two-arm machine decision and
    // checks only lowering/profile mechanics below that semantic boundary.

    let semantic_forms = eval_value(
        "(x86-lower-eq-cond-u64-forms 2 3 111 222)",
        &mut session,
    );
    assert_eq!(
        semantic_forms,
        "((mov-r64-imm64 rax 2) (mov-r64-imm64 rcx 3) (cmp-r64-r64 rax rcx) (jnz-rel8 11) (mov-r64-imm64 rax 111) (ret) (mov-r64-imm64 rax 222) (ret))"
    );

    let profile_forms = eval_value(
        "(x86-minimal-eq-cond-forms 2 3 111 222)",
        &mut session,
    );
    assert_eq!(profile_forms, semantic_forms);

    let families = eval_value(
        "(x86-minimal-eq-cond-observed-families 2 3 111 222)",
        &mut session,
    );
    assert_eq!(families, "(mov-r64-imm64 cmp-r64-r64 jnz-rel8 ret)");
    assert_eq!(
        eval_value("(x86-minimal-eq-cond-dependency-families)", &mut session),
        families
    );
}

#[test]
fn bounded_cond_profile_has_typed_jnz_atom_and_fails_closed_outside_disp8() {
    let mut session = profile_session();

    assert_eq!(eval_value("(x86-jnz-rel8 11)", &mut session), "(jnz-rel8 11)");
    assert_eq!(
        eval_value("(x86-jnz-rel8 128)", &mut session),
        "(rejected machine-operand disp8 128)"
    );
}

#[test]
fn bounded_cond_profile_reaches_bytes_only_through_closed_admission_and_round_trips() {
    let mut session = profile_session();
    let forms = eval_value(
        "(x86-minimal-eq-cond-forms 2 3 111 222)",
        &mut session,
    );
    let encoded = eval_value(
        "(x86-encode-admitted-program-or-reject (x86-minimal-eq-cond-forms 2 3 111 222))",
        &mut session,
    );

    assert!(encoded.starts_with('('), "expected encoded byte list, got {encoded}");
    assert!(
        !encoded.contains("rejected"),
        "conditional minimal profile must pass current closed admission: {encoded}"
    );

    let bytes = parse_byte_list(&encoded);
    let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
        .unwrap_or_else(|error| panic!("independent decoder rejected bounded COND witness {bytes:?}: {error}"));
    assert_eq!(render_forms(&decoded), forms);
}
