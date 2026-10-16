use sens::{eval_program, load_core_library, lower_program, parse, ExprKind, Session};
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
    eval_program(&source, session)
        .unwrap_or_else(|error| panic!("{} must load as ordinary sens: {error}", path.display()));
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
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/operands/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/atoms/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/projection/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-x86-64.lisp", &mut session);
    session
}

#[test]
fn current_d5_plus_builds_target_neutral_bounded_add_effect() {
    let parsed = parse("(додати 2 3)").expect("current PLUS source");
    let lowered = lower_program(&parsed);
    let ExprKind::DomainCall(identity, args) = &lowered[0].kind else {
        panic!("PLUS must lower to exact DomainCall");
    };
    assert_eq!((identity.width(), identity.packed_bits()), (5, 10));
    assert_eq!(args.len(), 2);

    let effect_source =
        fs::read_to_string(repo_root().join("lib/machine/effects/u64.lisp")).expect("effect");
    let lower = effect_source.to_ascii_lowercase();
    for forbidden in [
        "x86",
        "rax",
        "rcx",
        "mov-r64",
        "add-r64",
        "0x01",
        "modrm",
        "vex",
    ] {
        assert!(
            !lower.contains(forbidden),
            "target-neutral effect leaked target identity token {forbidden}"
        );
    }

    let mut session = machine_session();
    assert_eq!(
        eval_value("(machine-effect-bounded-u64-add 2 3)", &mut session),
        "(bounded-u64-add 2 3)"
    );
}

#[test]
fn bounded_add_effect_projects_to_same_forms_and_bytes_as_current_donor() {
    let mut session = machine_session();

    for (left, right) in [(2u64, 3u64), (0, u32::MAX as u64), (u32::MAX as u64, u32::MAX as u64)] {
        let effect = eval_value(
            &format!("(machine-effect-bounded-u64-add {left} {right})"),
            &mut session,
        );
        assert_eq!(effect, format!("(bounded-u64-add {left} {right})"));

        let projected = eval_value(
            &format!("(x86-project-machine-effect (quote {effect}))"),
            &mut session,
        );
        let donor = eval_value(
            &format!("(x86-lower-add-u64-forms {left} {right})"),
            &mut session,
        );
        assert_eq!(projected, donor, "projected forms must equal donor forms");

        let projected_bytes = eval_value(
            &format!("(x86-encode-machine-effect (quote {effect}))"),
            &mut session,
        );
        let donor_bytes = eval_value(
            &format!("(x86-encode-admitted-program (quote {donor}))"),
            &mut session,
        );
        assert_eq!(
            projected_bytes, donor_bytes,
            "effect projection must preserve current admitted bytes"
        );

        let bytes = parse_bytes(&projected_bytes);
        let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
            .unwrap_or_else(|error| panic!("independent decoder rejected {bytes:?}: {error}"));
        assert_eq!(
            render_decoded(&decoded),
            projected,
            "independent decode must reconstruct the projected forms"
        );
    }
}

#[test]
fn effect_and_projection_rejections_are_distinct_and_fail_closed() {
    let mut session = machine_session();

    for source in [
        "(machine-effect-bounded-u64-add -1 3)",
        "(machine-effect-bounded-u64-add 4294967296 3)",
        "(machine-effect-bounded-u64-add 2 4294967296)",
    ] {
        assert_eq!(eval_value(source, &mut session), "machine-effect-rejected");
    }

    assert_eq!(
        eval_value(
            "(x86-project-machine-effect (quote (unknown-effect 2 3)))",
            &mut session,
        ),
        "x86-projection-rejected"
    );
    assert_eq!(
        eval_value(
            "(x86-encode-machine-effect (quote (unknown-effect 2 3)))",
            &mut session,
        ),
        "x86-projection-rejected"
    );
}
