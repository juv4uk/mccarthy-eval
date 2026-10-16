use sens::{eval_program, eval_parsed_expressions, parse_mixed_exact_domain_machine_source, load_core_library, Session};
use std::fs;
use std::path::PathBuf;

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

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before machine atoms");
    load_lisp_file("lib/machine/block.lisp", &mut session);
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/operands/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/atoms/x86-64.lisp", &mut session);
    session
}

fn eval_value(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("machine expression failed: {source}: {error}"))
        .value
        .to_string()
}

#[test]
fn machine_atoms_project_exactly_to_current_admitted_forms() {
    let mut session = machine_session();

    for (source, expected) in [
        ("(x86-ret)", "(ret)"),
        (
            "(x86-mov-r64-imm64 (quote rax) 42)",
            "(mov-r64-imm64 rax 42)",
        ),
        (
            "(x86-add-r64-r64 (quote rax) (quote rcx))",
            "(add-r64-r64 rax rcx)",
        ),
        ("(x86-jnz-rel8 11)", "(jnz-rel8 11)"),
        (
            "(x86-mov-mem-disp8-r64 (quote rdi) 8 (quote rax))",
            "(mov-mem-disp8-r64 rdi 8 rax)",
        ),
        (
            "(x86-mov-r64-mem-disp8 (quote rax) (quote rdi) 8)",
            "(mov-r64-mem-disp8 rax rdi 8)",
        ),
    ] {
        assert_eq!(eval_value(source, &mut session), expected, "{source}");
    }
}

#[test]
fn machine_block_composition_is_deterministic_and_encodes_via_176_path() {
    let mut session = machine_session();

    let append_path = "(machine-block-append (machine-block-append (machine-block-append (machine-block-one (x86-mov-r64-imm64 (quote rax) 2)) (x86-mov-r64-imm64 (quote rcx) 3)) (x86-add-r64-r64 (quote rax) (quote rcx))) (x86-ret))";
    let concat_path = "(machine-block-concat (machine-block-concat (machine-block-one (x86-mov-r64-imm64 (quote rax) 2)) (machine-block-one (x86-mov-r64-imm64 (quote rcx) 3))) (machine-block-concat (machine-block-one (x86-add-r64-r64 (quote rax) (quote rcx))) (machine-block-one (x86-ret))))";

    let append_block = eval_value(append_path, &mut session);
    let concat_block = eval_value(concat_path, &mut session);
    assert_eq!(append_block, concat_block);
    assert_eq!(
        append_block,
        "((mov-r64-imm64 rax 2) (mov-r64-imm64 rcx 3) (add-r64-r64 rax rcx) (ret))"
    );

    let encoded = eval_value(
        &format!("(x86-encode-machine-block (quote {append_block}))"),
        &mut session,
    );
    assert_eq!(
        encoded,
        "(72 184 2 0 0 0 0 0 0 0 72 185 3 0 0 0 0 0 0 0 72 1 200 195)"
    );
}

#[test]
fn unadmitted_machine_atom_fails_closed_before_byte_materialization() {
    // #176 generalized mov-r64-imm64 admission to all 16 GPRs, so a
    // register-restriction gap no longer exists to exercise here. SHL is a
    // real x86-64 instruction, valid register, just not yet on #176's
    // admitted list -- the same shape of gap this test always meant to
    // exercise, via the same untyped `x86-unary-gpr64-form` constructor
    // push-r64/pop-r64/inc-r64/dec-r64 already use for a real mnemonic.
    let mut session = machine_session();
    assert_eq!(
        eval_value(
            "(x86-encode-machine-block (machine-block-one (x86-unary-gpr64-form (quote shl-r64) (quote rbx))))",
            &mut session,
        ),
        "(rejected unadmitted-machine-form (shl-r64 rbx))"
    );
}

#[test]
fn machine_atom_names_do_not_mint_public_semantic_surfaces() {
    for machine_only in ["x86-ret","x86-mov-r64-imm64","x86-add-r64-r64","x86-jnz-rel8","machine-block"] {
        assert!(
            sens::semantic_registry_export::semantic_id_for_admitted_surface(machine_only)
                .is_none(),
            "machine-only constructor {machine_only} must not become a semantic registry surface"
        );
    }
}
