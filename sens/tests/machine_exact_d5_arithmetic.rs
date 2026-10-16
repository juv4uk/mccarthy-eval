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
enum ArithmeticTail {
    Sub { destination: u8, source: u8 },
    Imul { destination: u8, source: u8 },
}

fn decode_arithmetic_tail(bytes: &[u8]) -> Option<ArithmeticTail> {
    if bytes.last().copied()? != 0xC3 {
        return None;
    }

    // The bounded witnesses load two imm64 values first: 10 bytes each.
    let tail = bytes.get(20..bytes.len() - 1)?;
    match tail {
        [rex, 0x29, modrm] if (rex & 0xF8) == 0x48 && (modrm >> 6) == 0b11 => {
            let rex_r = (rex >> 2) & 1;
            let rex_b = rex & 1;
            let source = ((modrm >> 3) & 0b111) | (rex_r << 3);
            let destination = (modrm & 0b111) | (rex_b << 3);
            Some(ArithmeticTail::Sub {
                destination,
                source,
            })
        }
        [rex, 0x0F, 0xAF, modrm] if (rex & 0xF8) == 0x48 && (modrm >> 6) == 0b11 => {
            let rex_r = (rex >> 2) & 1;
            let rex_b = rex & 1;
            let destination = ((modrm >> 3) & 0b111) | (rex_r << 3);
            let source = (modrm & 0b111) | (rex_b << 3);
            Some(ArithmeticTail::Imul {
                destination,
                source,
            })
        }
        _ => None,
    }
}

#[test]
fn current_surfaces_lower_to_exact_d5_difference_and_times() {
    for (source, expected_bits) in [("(відняти 9 4)", 0b01011), ("(помножити 6 7)", 0b10110)] {
        let lowered = lower_program(&parse(source).expect("current D5 source"));
        let ExprKind::DomainCall(identity, args) = &lowered[0].kind else {
            panic!("{source} must lower to exact DomainCall");
        };
        assert_eq!(identity.width(), 5);
        assert_eq!(identity.packed_bits(), expected_bits);
        assert_eq!(args.len(), 2);
    }
}

#[test]
fn exact_d5_difference_and_times_reach_admitted_x86_bytes() {
    let mut session = machine_session();

    for (packed_bits, expected_capability) in [
        (
            11,
            "((integer-subtract bounded-u64 no-underflow))",
        ),
        (
            22,
            "((integer-multiply bounded-u32-inputs u64-result))",
        ),
    ] {
        let capability = eval_program(
            &format!("(machine-capabilities-for-domain 5 {packed_bits})"),
            &mut session,
        )
        .expect("current D5 capability")
        .value
        .to_string();
        assert_eq!(capability, expected_capability);
    }

    let sub_forms = eval_program(
        "(x86-lower-current-binary-u64-forms 5 11 9 4)",
        &mut session,
    )
    .expect("D5 DIFFERENCE forms")
    .value
    .to_string();
    assert_eq!(
        sub_forms,
        "((mov-r64-imm64 rax 9) (mov-r64-imm64 rcx 4) (sub-r64-r64 rax rcx) (ret))"
    );

    let sub_bytes = eval_program(
        "(x86-encode-current-binary-u64 5 11 9 4)",
        &mut session,
    )
    .expect("D5 DIFFERENCE bytes")
    .value
    .to_string();
    assert_eq!(
        decode_arithmetic_tail(&parse_bytes(&sub_bytes)),
        Some(ArithmeticTail::Sub {
            destination: 0,
            source: 1,
        })
    );

    let times_forms = eval_program(
        "(x86-lower-current-binary-u64-forms 5 22 6 7)",
        &mut session,
    )
    .expect("D5 TIMES forms")
    .value
    .to_string();
    assert_eq!(
        times_forms,
        "((mov-r64-imm64 rax 6) (mov-r64-imm64 rcx 7) (imul-r64-r64 rax rcx) (ret))"
    );

    let times_bytes = eval_program(
        "(x86-encode-current-binary-u64 5 22 6 7)",
        &mut session,
    )
    .expect("D5 TIMES bytes")
    .value
    .to_string();
    assert_eq!(
        decode_arithmetic_tail(&parse_bytes(&times_bytes)),
        Some(ArithmeticTail::Imul {
            destination: 0,
            source: 1,
        })
    );
}

#[test]
fn exact_d5_fast_paths_fail_closed_when_u64_would_change_meaning() {
    let mut session = machine_session();

    for form in [
        "(x86-encode-current-binary-u64 5 11 1 2)",
        "(x86-encode-current-binary-u64 5 22 18446744073709551615 2)",
        "(x86-encode-current-binary-u64 5 10 18446744073709551615 1)",
        "(x86-encode-current-binary-u64 5 11 -1 1)",
    ] {
        let actual = eval_program(form, &mut session)
            .unwrap_or_else(|error| panic!("guard expression failed: {form}: {error}"))
            .value
            .to_string();
        assert_eq!(
            actual, "exact-d5-fallback-required",
            "{form} must not inherit modulo-u64 semantics"
        );
    }
}

#[test]
fn current_profile_only_admits_difference_and_times_after_witnesses_exist() {
    let profile = fs::read_to_string(repo_root().join("lib/machine/profile/current-domain-x86-64.lisp"))
        .expect("current exact-domain machine profile");

    assert!(profile.contains("(5 11 fast-path \"SUB / u64, left>=right\")"));
    assert!(profile.contains("(5 22 fast-path \"IMUL / u32 inputs -> exact u64 result\")"));
    assert!(!profile.contains("00001101 fast-path"));
    assert!(!profile.contains("00001110 fast-path"));
}
