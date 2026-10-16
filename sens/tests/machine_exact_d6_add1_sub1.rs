use sens::{eval_program, load_core_library, Bit6, CoreD6, DomainIdentity, Session};
use std::fs;
use std::path::PathBuf;

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

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/capability-axis.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-x86-64.lisp", &mut session);
    session
}

#[test]
fn exact_d6_add1_sub1_have_width_safe_machine_capabilities() {
    let mut session = machine_session();

    assert_eq!(
        eval_program("(machine-capabilities-for-domain 6 14)", &mut session)
            .expect("D6 ADD1 capability")
            .value
            .to_string(),
        "((integer-increment bounded-u64 no-overflow))"
    );
    assert_eq!(
        eval_program("(machine-capabilities-for-domain 6 15)", &mut session)
            .expect("D6 SUB1 capability")
            .value
            .to_string(),
        "((integer-decrement bounded-u64 no-underflow))"
    );

    for form in [
        "(machine-capabilities-for-domain 5 14)",
        "(machine-capabilities-for-domain 5 15)",
        "(machine-capabilities-for-domain 7 14)",
        "(machine-capabilities-for-domain 8 15)",
    ] {
        assert_eq!(
            eval_program(form, &mut session)
                .unwrap_or_else(|error| panic!("cross-width control failed: {form}: {error}"))
                .value
                .to_string(),
            "()",
            "{form} must not inherit D6 machine meaning"
        );
    }
}


#[test]
fn admitted_d6_domain_identity_drives_machine_key_without_source_width_recovery() {
    let mut session = machine_session();

    for (bits, input, expected) in [
        (0b001110, 41u64, "(72 184 41 0 0 0 0 0 0 0 72 185 1 0 0 0 0 0 0 0 72 1 200 195)"),
        (0b001111, 41u64, "(72 184 41 0 0 0 0 0 0 0 72 185 1 0 0 0 0 0 0 0 72 41 200 195)"),
    ] {
        let identity = DomainIdentity::D6(CoreD6::from_word(
            Bit6::new(bits).expect("D6 identity bits"),
        ));
        assert!(
            identity.core_operation().is_some(),
            "D6:{bits:06b} must be admitted before machine dispatch"
        );

        let form = format!(
            "(x86-encode-current-d6-unary-u64 {} {} {})",
            identity.width(),
            identity.packed_bits(),
            input
        );
        let bytes = eval_program(&form, &mut session)
            .unwrap_or_else(|error| panic!("exact DomainIdentity machine route failed: {error}"))
            .value
            .to_string();
        assert_eq!(bytes, expected, "D6:{bits:06b}");
    }
}

#[test]
fn exact_d6_add1_sub1_reuse_admitted_add_sub_forms_and_bytes() {
    let mut session = machine_session();

    let add_forms = eval_program(
        "(x86-lower-current-d6-unary-u64-forms 6 14 41)",
        &mut session,
    )
    .expect("D6 ADD1 forms")
    .value
    .to_string();
    assert_eq!(
        add_forms,
        "((mov-r64-imm64 rax 41) (mov-r64-imm64 rcx 1) (add-r64-r64 rax rcx) (ret))"
    );

    let add_bytes = eval_program(
        "(x86-encode-current-d6-unary-u64 6 14 41)",
        &mut session,
    )
    .expect("D6 ADD1 bytes")
    .value
    .to_string();
    assert_eq!(
        add_bytes,
        "(72 184 41 0 0 0 0 0 0 0 72 185 1 0 0 0 0 0 0 0 72 1 200 195)"
    );

    let sub_forms = eval_program(
        "(x86-lower-current-d6-unary-u64-forms 6 15 41)",
        &mut session,
    )
    .expect("D6 SUB1 forms")
    .value
    .to_string();
    assert_eq!(
        sub_forms,
        "((mov-r64-imm64 rax 41) (mov-r64-imm64 rcx 1) (sub-r64-r64 rax rcx) (ret))"
    );

    let sub_bytes = eval_program(
        "(x86-encode-current-d6-unary-u64 6 15 41)",
        &mut session,
    )
    .expect("D6 SUB1 bytes")
    .value
    .to_string();
    assert_eq!(
        sub_bytes,
        "(72 184 41 0 0 0 0 0 0 0 72 185 1 0 0 0 0 0 0 0 72 41 200 195)"
    );
}

#[test]
fn exact_d6_unary_fast_paths_fail_closed_outside_exact_u64_rectangle() {
    let mut session = machine_session();

    for form in [
        "(x86-encode-current-d6-unary-u64 6 14 -1)",
        "(x86-encode-current-d6-unary-u64 6 14 1/2)",
        "(x86-encode-current-d6-unary-u64 6 14 18446744073709551615)",
        "(x86-encode-current-d6-unary-u64 6 15 0)",
        "(x86-encode-current-d6-unary-u64 6 15 -1)",
        "(x86-encode-current-d6-unary-u64 6 15 1/2)",
        "(x86-encode-current-d6-unary-u64 6 15 18446744073709551616)",
    ] {
        assert_eq!(
            eval_program(form, &mut session)
                .unwrap_or_else(|error| panic!("D6 guard failed: {form}: {error}"))
                .value
                .to_string(),
            "exact-d6-fallback-required",
            "{form} must preserve exact D6/D5 arithmetic semantics"
        );
    }

    for form in [
        "(x86-encode-current-d6-unary-u64 5 14 41)",
        "(x86-encode-current-d6-unary-u64 5 15 41)",
    ] {
        assert_eq!(
            eval_program(form, &mut session)
                .unwrap_or_else(|error| panic!("wrong-domain guard failed: {form}: {error}"))
                .value
                .to_string(),
            "unsupported-current-domain-d6-unary-u64"
        );
    }
}

#[test]
fn current_profile_records_only_exact_d6_width_safe_rows() {
    let profile =
        fs::read_to_string(repo_root().join("lib/machine/profile/current-domain-x86-64.lisp"))
            .expect("current exact-domain machine profile");

    assert!(profile.contains(
        "(6 14 fast-path \"ADD / exact u64 input <= u64-2, RHS=1\")"
    ));
    assert!(profile.contains(
        "(6 15 fast-path \"SUB / exact u64 input >=1, RHS=1\")"
    ));
    assert!(!profile.contains("(5 14 fast-path"));
    assert!(!profile.contains("(5 15 fast-path"));
    assert!(!profile.contains("semantic-registry"));
}
