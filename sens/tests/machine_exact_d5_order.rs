use sens::{
    eval_program, load_core_library, lower_program, parse, ExprKind, Session,
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


fn parse_bytes(rendered: &str) -> Vec<u8> {
    rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .map(|token| token.parse::<u8>().expect("machine byte"))
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct OrderTail {
    condition_code: u8,
    set_register: u8,
    result_register: u8,
}

fn decode_order_tail(bytes: &[u8]) -> Option<OrderTail> {
    // Two MOV r64,imm64 instructions = 20 bytes.
    let tail = bytes.get(20..)?;
    let [cmp_rex, cmp_opcode, cmp_modrm,
         set_prefix, set_opcode, set_modrm,
         movzx_rex, movzx_prefix, movzx_opcode, movzx_modrm,
         ret] = tail
    else {
        return None;
    };

    // CMP rax, rcx.
    if (*cmp_rex, *cmp_opcode, *cmp_modrm) != (0x48, 0x39, 0xC8) {
        return None;
    }
    if *set_prefix != 0x0F || !matches!(*set_opcode, 0x9C | 0x9F) {
        return None;
    }
    if (*set_modrm >> 6) != 0b11 {
        return None;
    }
    let condition_code = set_opcode - 0x90;
    let set_register = set_modrm & 0b111;

    // MOVZX rax, al.
    if (*movzx_rex, *movzx_prefix, *movzx_opcode) != (0x48, 0x0F, 0xB6)
        || (*movzx_modrm >> 6) != 0b11
    {
        return None;
    }
    let result_register = (movzx_modrm >> 3) & 0b111;
    if (movzx_modrm & 0b111) != set_register || *ret != 0xC3 {
        return None;
    }

    Some(OrderTail {
        condition_code,
        set_register,
        result_register,
    })
}

#[test]
fn current_order_surfaces_lower_to_exact_d5_identities() {
    for (source, expected_bits) in [("(менше? 2 3)", 0b11010), ("(більше? 3 2)", 0b11011)] {
        let lowered = lower_program(&parse(source).expect("current D5 order source"));
        let ExprKind::DomainCall(identity, args) = &lowered[0].kind else {
            panic!("{source} must lower to exact DomainCall");
        };
        assert_eq!(identity.width(), 5);
        assert_eq!(identity.packed_bits(), expected_bits);
        assert_eq!(args.len(), 2);
    }
}

#[test]
fn bounded_order_predicates_reach_admitted_internal_bit_bytes() {
    let mut session = machine_session();

    for (packed_bits, capability) in [
        (
            26,
            "((integer-order-less bounded-nonnegative-i63 internal-bit-d1-boundary))",
        ),
        (
            27,
            "((integer-order-greater bounded-nonnegative-i63 internal-bit-d1-boundary))",
        ),
    ] {
        assert_eq!(
            eval_program(
                &format!("(machine-capabilities-for-domain 5 {packed_bits})"),
                &mut session,
            )
            .expect("order capability")
            .value
            .to_string(),
            capability
        );
    }

    let less_forms = eval_program(
        "(x86-lower-order-i64-forms 5 26 2 3)",
        &mut session,
    )
    .expect("LESSP forms")
    .value
    .to_string();
    assert_eq!(
        less_forms,
        "((mov-r64-imm64 rax 2) (mov-r64-imm64 rcx 3) (cmp-r64-r64 rax rcx) (setl-r8 al) (movzx-r64-r8 rax al) (ret))"
    );

    let less_bytes = eval_program(
        "(x86-encode-current-order-bit 5 26 2 3)",
        &mut session,
    )
    .expect("LESSP bytes")
    .value
    .to_string();
    assert_eq!(
        decode_order_tail(&parse_bytes(&less_bytes)),
        Some(OrderTail {
            condition_code: 12,
            set_register: 0,
            result_register: 0,
        })
    );

    let greater_forms = eval_program(
        "(x86-lower-order-i64-forms 5 27 3 2)",
        &mut session,
    )
    .expect("GREATERP forms")
    .value
    .to_string();
    assert_eq!(
        greater_forms,
        "((mov-r64-imm64 rax 3) (mov-r64-imm64 rcx 2) (cmp-r64-r64 rax rcx) (setg-r8 al) (movzx-r64-r8 rax al) (ret))"
    );

    let greater_bytes = eval_program(
        "(x86-encode-current-order-bit 5 27 3 2)",
        &mut session,
    )
    .expect("GREATERP bytes")
    .value
    .to_string();
    assert_eq!(
        decode_order_tail(&parse_bytes(&greater_bytes)),
        Some(OrderTail {
            condition_code: 15,
            set_register: 0,
            result_register: 0,
        })
    );
}

#[test]
fn order_native_slice_fails_closed_outside_first_proved_integer_rectangle() {
    let mut session = machine_session();

    for form in [
        "(x86-encode-current-order-bit 5 26 -1 0)",
        "(x86-encode-current-order-bit 5 27 0 -1)",
        "(x86-encode-current-order-bit 5 26 9223372036854775808 1)",
        "(x86-encode-current-order-bit 5 27 1 9223372036854775808)",
    ] {
        assert_eq!(
            eval_program(form, &mut session)
                .unwrap_or_else(|error| panic!("order guard failed: {form}: {error}"))
                .value
                .to_string(),
            "exact-d5-fallback-required",
            "{form}"
        );
    }

    assert_eq!(
        eval_program(
            "(x86-encode-current-order-bit 3 5 2 3)",
            &mut session,
        )
        .expect("wrong exact identity fails closed")
        .value
        .to_string(),
        "unsupported-current-domain-order-i64"
    );
}

#[test]
fn order_machine_proof_does_not_publish_a_numeric_boolean_language_api() {
    let source =
        fs::read_to_string(repo_root().join("lib/machine/lowering/semantic-x86-64.lisp"))
            .expect("semantic x86 lowering");

    let start = source
        .find("; #4018 exact D5 order-predicate machine proof.")
        .expect("#4018 order proof block");
    let end = source[start..]
        .find("; #3996 exact-domain structural D3 dispatcher.")
        .map(|offset| start + offset)
        .unwrap_or(source.len());
    let order_block = &source[start..end];

    assert!(order_block.contains("(00001001 x86-encode-current-order-bit"));
    for forbidden in [
        "x86-call-current-order",
        "numeric-boolean",
        "machine-predicate-number",
    ] {
        assert!(
            !order_block.contains(forbidden),
            "machine order proof must not publish {forbidden}"
        );
    }

    let profile =
        fs::read_to_string(repo_root().join("lib/machine/profile/current-domain-x86-64.lisp"))
            .expect("current profile");
    assert!(profile.contains("(5 26 proof \"CMP+SETL+MOVZX / internal bit; D1 boundary\")"));
    assert!(profile.contains("(5 27 proof \"CMP+SETG+MOVZX / internal bit; D1 boundary\")"));
    assert!(!profile.contains("00011010 direct"));
    assert!(!profile.contains("00011011 direct"));
}
