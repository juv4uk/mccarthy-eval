use sens::{
    eval_parsed_expressions, eval_program, load_core_library, parse_mixed_exact_domain_machine_source,
    register_capability, Environment, Exactness, Expr, LanguageError, Session, Span, Value,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_lisp_file(path: &str, session: &mut Session) {
    assert!(
        path.starts_with("lib/machine/"),
        "machine-source reader is restricted to lib/machine/** fixtures: {path}"
    );
    let source = {
        let file_path = repo_root().join(path);
        fs::read_to_string(&file_path)
            .unwrap_or_else(|error| panic!("{} must exist: {error}", file_path.display()))
    };
    let expressions = parse_mixed_exact_domain_machine_source(path, &source)
        .unwrap_or_else(|error| panic!("{path} must parse as exact-domain machine source: {error}"));
    eval_parsed_expressions(&expressions, session)
        .unwrap_or_else(|error| panic!("{path} must load through the machine-source reader: {error}"));
}

fn spy_executor(
    _arguments: &[Expr],
    _environment: &Environment,
    _span: Span,
) -> Result<Value, LanguageError> {
    EXECUTOR_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(Value::Number(999.0, Exactness::Exact))
}


#[test]
fn exhausted_instruction_patterns_return_exact_d1_no_not_structural_empty() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("ратифіковане ядро");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);

    // Порожній перелік шаблонів — це предикат D1:0, а не структурне D3:000.
    for source in [
        "(x86-admitted-instruction-against? (00000001 ()) (00000001 ud2))",
        "(x86-admitted-instruction-against? (00000001 ()) (00000001 ()))",
    ] {
        let result = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(false),
            "неприпустима інструкція не може повертати структурне ()"
        );
    }
}

#[test]
fn x86_exact_integer_wire_checks_return_typed_d1() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("ратифіковане ядро");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);

    // Перевірка справді проходить крізь лексичні прив'язки wire/rest.
    // Недопустимі числа не можна заміняти структурним () чи Number 0.
    for (source, expected) in [
        ("(x86-admission-exact-integer? 7)", true),
        ("(x86-admission-exact-integer? -7)", true),
        ("(x86-admission-exact-integer? (00000001 not-an-integer))", false),
    ] {
        let result = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(expected),
            "канонічне ціле число має відповідати лише точним D1:1/D1:0: {source}"
        );
    }
}

#[test]
fn x86_pattern_mismatches_are_exact_d1_no_not_empty() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("ратифіковане ядро");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);

    let cases = [
        ("(x86-admission-pattern-match? (00000001 (ret)) (00000001 (ud2)))", false),
        ("(x86-admission-pattern-match? (00000001 (ret)) (00000001 ret))", false),
        ("(x86-admission-pattern-match? (00000001 (ret)) (00000001 (ret extra)))", false),
        ("(x86-admission-pattern-match? (00000001 (ret)) (00000001 (ret)))", true),
        ("(x86-admission-pattern-match? (00000001 (mov-r64-imm64 register immediate)) (00000001 (mov-r64-imm64 rax 7)))", true),
        ("(x86-admission-pattern-match? (00000001 (mov-r64-imm64 register immediate)) (00000001 (mov-r64-imm64 notareg 7)))", false),
        ("(x86-admission-pattern-match? (00000001 (vaddps-xmm-xmm-xmm xmm-register xmm-register xmm-register)) (00000001 (vaddps-xmm-xmm-xmm xmm0 xmm1 xmm2)))", true),
        ("(x86-admission-pattern-match? (00000001 (vaddps-xmm-xmm-xmm xmm-register xmm-register xmm-register)) (00000001 (vaddps-xmm-xmm-xmm xmm0 notaxmm xmm2)))", false),
    ];

    for (source, expected) in cases {
        let result = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(expected),
            "pattern matching must return exact D1, never host truthiness, structural empty, or Number: {source}"
        );
    }
}



#[test]
fn x86_xmm_name_predicate_returns_exact_d1_for_known_and_unknown_names() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("ратифіковане ядро");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/operands/x86-64.lisp", &mut session);

    for (source, expected) in [
        ("(x86-xmm-name? xmm0)", true),
        ("(x86-xmm-name? xmm10)", true),
        ("(x86-xmm-name? xmm15)", true),
        ("(x86-xmm-name? notaxmm)", false),
        ("(x86-xmm-name? 42)", false),
    ] {
        let result = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(expected),
            "XMM name predicate must return exact D1: {source}"
        );
    }
}


#[test]
fn x86_admitted_program_rejects_ud2_with_typed_d1_no() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("ратифіковане ядро");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);

    for program in [
        "(x86-admitted-program? (00000001 ((ud2))))",
        "(x86-admitted-program? (00000001 ((mov-r64-imm64 notareg 1))))",
    ] {
        let result = eval_program(program, &mut session)
            .unwrap_or_else(|error| panic!("{program}: {error}"));
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(false),
            "відмова допуску зобов'язана мати D1:0, а не D3:000"
        );
    }
}

#[test]
fn canonical_machine_gateway_rejects_raw_bytes_register_bypass_and_truncation_before_host() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before admission adversaries");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);

    register_capability("native-call-u64-raw", spy_executor);

    for (request, expected) in [
        (
            "(x86-call-admitted-u64 (quote (15 11)) 0)",
            "(rejected unadmitted-machine-form 15)",
        ),
        (
            // #176 generalized mov-r64-imm64 admission to all 16 GPRs
            // (previously rax/rcx only), so a made-up register name is now
            // the register slot's negative witness instead of a real GPR.
            "(x86-call-admitted-u64 (quote ((mov-r64-imm64 notareg 1))) 0)",
            "(rejected unadmitted-machine-form (mov-r64-imm64 notareg 1))",
        ),
        (
            "(x86-call-admitted-u64 (quote ((mov-r64-imm64 rax))) 0)",
            "(rejected unadmitted-machine-form (mov-r64-imm64 rax))",
        ),
        (
            // `register` is an operand-slot wildcard admitting only the 16
            // real GPR names; a made-up symbol must still fail closed, not
            // be silently accepted because *some* atom sits in that slot.
            "(x86-call-admitted-u64 (quote ((add-r64-r64 rax notareg))) 0)",
            "(rejected unadmitted-machine-form (add-r64-r64 rax notareg))",
        ),
        (
            // #176's generalized mov-mem-disp8 admission: 128 is one past
            // the signed disp8 maximum (127) and must fail closed, not be
            // silently truncated or wrapped into something encodable.
            "(x86-call-admitted-u64 (quote ((mov-r64-mem-disp8 rax rdi 128))) 0)",
            "(rejected unadmitted-machine-form (mov-r64-mem-disp8 rax rdi 128))",
        ),
        (
            // -129 is one past the signed disp8 minimum (-128).
            "(x86-call-admitted-u64 (quote ((mov-mem-disp8-r64 rdi -129 rax))) 0)",
            "(rejected unadmitted-machine-form (mov-mem-disp8-r64 rdi -129 rax))",
        ),
        (
            // The `disp8` wildcard must fail closed on a non-numeric atom
            // rather than erroring out of the comparison operators that
            // would otherwise be applied to it.
            "(x86-call-admitted-u64 (quote ((mov-mem-disp8-r64 rdi notanumber rax))) 0)",
            "(rejected unadmitted-machine-form (mov-mem-disp8-r64 rdi notanumber rax))",
        ),
        (
            // A made-up base register in the mem-disp8 form must fail
            // closed the same way an invalid register does elsewhere.
            "(x86-call-admitted-u64 (quote ((mov-r64-mem-disp8 rax notareg 0))) 0)",
            "(rejected unadmitted-machine-form (mov-r64-mem-disp8 rax notareg 0))",
        ),
        (
            // Jcc rel8's disp8 slot fails closed the same way MOV's does:
            // one past the signed max.
            "(x86-call-admitted-u64 (quote ((jz-rel8 128))) 0)",
            "(rejected unadmitted-machine-form (jz-rel8 128))",
        ),
        (
            // A made-up condition mnemonic must fail closed, not be
            // silently confused with a real one.
            "(x86-call-admitted-u64 (quote ((jzzz-rel8 0))) 0)",
            "(rejected unadmitted-machine-form (jzzz-rel8 0))",
        ),
        (
            // JMP rel8's disp8 slot fails closed the same way Jcc's does:
            // one past the signed min.
            "(x86-call-admitted-u64 (quote ((jmp-rel8 -129))) 0)",
            "(rejected unadmitted-machine-form (jmp-rel8 -129))",
        ),
    ] {
        EXECUTOR_CALLS.store(0, Ordering::SeqCst);
        let result = eval_program(request, &mut session)
            .expect("unadmitted machine data must be rejected as Lisp data, not host failure");
        assert_eq!(result.value.to_string(), expected);
        assert_eq!(
            EXECUTOR_CALLS.load(Ordering::SeqCst),
            0,
            "rejected raw/malformed machine data must never reach the host executor: {request}"
        );
    }
}
