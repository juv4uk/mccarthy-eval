use sens::{
    eval_parsed_expressions, eval_program, load_core_library, lower_program, parse, Bit5, CoreD5,
    DomainIdentity, Expr, ExprKind, Session, Span,
};
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

fn exact_d5_call(bits: u8, args: &str, session: &mut Session) -> String {
    let mut parsed = parse(&format!("(__d5_probe__ {args})")).expect("probe payload");
    let mut form = parsed.remove(0);
    let ExprKind::List(items) = form.kind else {
        panic!("probe list");
    };
    let mut items = items.to_vec();
    items[0] = Expr {
        kind: ExprKind::DomainIdentity(DomainIdentity::D5(CoreD5::from_word(
            Bit5::new(bits).expect("D5 bits"),
        ))),
        span: Span::default(),
    };
    form.kind = ExprKind::List(items.into());
    eval_parsed_expressions(&[form], session)
        .expect("exact D5 call")
        .value
        .to_string()
}

fn parse_bytes(rendered: &str) -> Vec<u8> {
    rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .map(|token| token.parse::<u8>().expect("machine byte"))
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct IdivTail {
    divisor_register: u8,
}

fn decode_cqo_idiv_tail(bytes: &[u8]) -> Option<IdivTail> {
    // Two MOV r64,imm64 forms consume 20 bytes.
    let tail = bytes.get(20..)?;
    let [cqo_rex, cqo_opcode, idiv_rex, idiv_opcode, modrm, ret] = tail else {
        return None;
    };
    if (*cqo_rex, *cqo_opcode, *ret) != (0x48, 0x99, 0xC3) {
        return None;
    }
    if *idiv_opcode != 0xF7 || (*idiv_rex & 0xF8) != 0x48 || (*modrm >> 6) != 0b11 {
        return None;
    }
    if ((*modrm >> 3) & 0b111) != 0b111 {
        return None;
    }
    let rex_b = *idiv_rex & 1;
    let divisor_register = (*modrm & 0b111) | (rex_b << 3);
    Some(IdivTail { divisor_register })
}

#[test]
fn current_quotient_surface_lowers_to_exact_d5_identity() {
    let lowered = lower_program(&parse("(частка 6 3)").expect("current QUOTIENT source"));
    let ExprKind::DomainCall(identity, args) = &lowered[0].kind else {
        panic!("current QUOTIENT must lower to exact DomainCall");
    };
    assert_eq!(identity.width(), 5);
    assert_eq!(identity.packed_bits(), 0b10111);
    assert_eq!(args.len(), 2);
}

#[test]
fn exact_d5_quotient_semantics_exist_beyond_the_native_proof_rectangle() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core");

    for (args, expected) in [("6 3", "2"), ("1 2", "1/2"), ("5 5", "1")] {
        assert_eq!(
            exact_d5_call(0b10111, args, &mut session),
            expected,
            "exact D5 QUOTIENT semantics for {args}"
        );
    }
}

#[test]
fn equal_positive_i64_quotient_reaches_admitted_idiv_bytes() {
    let mut session = machine_session();

    let capability = eval_program(
        "(machine-capabilities-for-domain 5 23)",
        &mut session,
    )
    .expect("D5 QUOTIENT capability")
    .value
    .to_string();
    assert_eq!(
        capability,
        "((integer-quotient bounded-positive-i64 equal-operands-only))"
    );

    let forms = eval_program(
        "(x86-lower-current-quotient-i64-forms 5 23 5 5)",
        &mut session,
    )
    .expect("bounded QUOTIENT forms")
    .value
    .to_string();
    assert_eq!(
        forms,
        "((mov-r64-imm64 rax 5) (mov-r64-imm64 rcx 5) (cqo) (idiv-r64 rcx) (ret))"
    );

    let rendered = eval_program(
        "(x86-encode-current-quotient-i64 5 23 5 5)",
        &mut session,
    )
    .expect("bounded QUOTIENT bytes")
    .value
    .to_string();
    let decoded = decode_cqo_idiv_tail(&parse_bytes(&rendered)).expect("CQO+IDIV tail");
    assert_eq!(decoded, IdivTail { divisor_register: 1 });
}

#[test]
fn quotient_native_slice_fails_closed_outside_the_proved_equal_operand_relation() {
    let mut session = machine_session();

    for form in [
        "(x86-encode-current-quotient-i64 5 23 6 3)",
        "(x86-encode-current-quotient-i64 5 23 1 2)",
        "(x86-encode-current-quotient-i64 5 23 0 0)",
        "(x86-encode-current-quotient-i64 5 23 -5 -5)",
        "(x86-encode-current-quotient-i64 5 23 9223372036854775808 9223372036854775808)",
    ] {
        let actual = eval_program(form, &mut session)
            .unwrap_or_else(|error| panic!("guard expression failed: {form}: {error}"))
            .value
            .to_string();
        assert_eq!(
            actual, "exact-d5-fallback-required",
            "{form} is outside the first proved IDIV slice"
        );
    }

    assert_eq!(
        eval_program(
            "(x86-encode-current-quotient-i64 5 22 5 5)",
            &mut session,
        )
        .expect("wrong identity must fail closed")
        .value
        .to_string(),
        "unsupported-current-domain-quotient-i64"
    );
}

#[test]
fn current_profile_admits_only_the_witnessed_quotient_slice() {
    let profile = fs::read_to_string(repo_root().join("lib/machine/profile/current-domain-x86-64.lisp"))
        .expect("current exact-domain machine profile");

    assert!(
        profile.contains("(5 23 fast-path \"CQO+IDIV / positive i64 equal operands\")")
    );
    assert!(
        !profile.contains("00010100 fast-path"),
        "historical byte SID QUOTIENT must not become current profile authority"
    );
}
