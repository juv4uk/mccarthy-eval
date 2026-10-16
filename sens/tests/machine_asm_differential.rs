use sens::{eval_parsed_expressions, eval_program, load_core_library, parse_mixed_exact_domain_machine_source, Session};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_lisp_file(path: &str, session: &mut Session) {
    assert!(
        path.starts_with("lib/machine/"),
        "machine-source reader is restricted to lib/machine/** fixtures: {path}"
    );
    let file_path = repo_root().join(path);
    let source = fs::read_to_string(&file_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", file_path.display()));
    let expressions = parse_mixed_exact_domain_machine_source(path, &source)
        .unwrap_or_else(|error| panic!("{path} must parse as exact-domain machine source: {error}"));
    eval_parsed_expressions(&expressions, session)
        .unwrap_or_else(|error| panic!("{path} must load through exact-domain reader: {error}"));
}

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before external assembler witness");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    session
}

fn parse_byte_list(rendered: &str) -> Vec<u8> {
    rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| token.parse().expect("encoded machine byte must fit u8"))
        .collect()
}

fn sens_bytes(form: &str, session: &mut Session) -> Vec<u8> {
    let source = format!("(x86-encode-admitted-program (quote ({form})))");
    let rendered = eval_program(&source, session)
        .unwrap_or_else(|error| panic!("SENS encoder rejected {form}: {error}"))
        .value
        .to_string();
    parse_byte_list(&rendered)
}

fn run(command: &str, args: &[&str]) -> Output {
    Command::new(command)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("required external witness tool {command} is unavailable: {error}"))
}

fn run_ok(command: &str, args: &[&str]) -> Output {
    let output = run(command, args);
    assert!(
        output.status.success(),
        "{command} {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn tool_version(command: &str, args: &[&str]) -> String {
    let output = run_ok(command, args);
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    text.lines().next().unwrap_or("<no version line>").to_string()
}

fn extract_text(object: &Path, binary: &Path) -> Vec<u8> {
    let object_s = object.to_string_lossy();
    let binary_s = binary.to_string_lossy();
    run_ok(
        "objcopy",
        &[
            "-O",
            "binary",
            "--only-section=.text",
            object_s.as_ref(),
            binary_s.as_ref(),
        ],
    );
    fs::read(binary).unwrap_or_else(|error| panic!("{} must be readable: {error}", binary.display()))
}

fn gas_bytes(dir: &Path, name: &str, body: &str) -> Vec<u8> {
    let source = dir.join(format!("{name}.gas.s"));
    let object = dir.join(format!("{name}.gas.o"));
    let binary = dir.join(format!("{name}.gas.bin"));
    fs::write(
        &source,
        format!(".intel_syntax noprefix\n.text\n.global witness\nwitness:\n    {body}\n"),
    )
    .expect("write GAS witness source");

    let source_s = source.to_string_lossy();
    let object_s = object.to_string_lossy();
    run_ok("as", &["--64", "-o", object_s.as_ref(), source_s.as_ref()]);
    extract_text(&object, &binary)
}

fn nasm_bytes(dir: &Path, name: &str, body: &str) -> Vec<u8> {
    let source = dir.join(format!("{name}.nasm.asm"));
    let object = dir.join(format!("{name}.nasm.o"));
    let binary = dir.join(format!("{name}.nasm.bin"));
    fs::write(
        &source,
        format!("bits 64\nsection .text\nglobal witness\nwitness:\n    {body}\n"),
    )
    .expect("write NASM witness source");

    let source_s = source.to_string_lossy();
    let object_s = object.to_string_lossy();
    run_ok(
        "nasm",
        &["-f", "elf64", "-o", object_s.as_ref(), source_s.as_ref()],
    );
    extract_text(&object, &binary)
}

struct Case {
    name: &'static str,
    form: &'static str,
    gas: &'static str,
    nasm: &'static str,
}

#[test]
#[ignore = "external GAS/NASM witness; run in machine-asm-differential workflow"]
fn gas_and_nasm_match_lisp_owned_encoder_for_admitted_corpus() {
    let gas_version = tool_version("as", &["--version"]);
    let nasm_version = tool_version("nasm", &["-v"]);
    let objcopy_version = tool_version("objcopy", &["--version"]);

    eprintln!(
        "external witness tools: {gas_version} | {nasm_version} | {objcopy_version}"
    );

    assert!(
        gas_version.ends_with(" 2.42"),
        "review #4344 evidence before changing pinned GAS 2.42: {gas_version}"
    );
    assert_eq!(
        nasm_version, "NASM version 2.16.01",
        "review #4344 evidence before changing pinned NASM version"
    );
    assert!(
        objcopy_version.ends_with(" 2.42"),
        "review #4344 evidence before changing pinned objcopy 2.42: {objcopy_version}"
    );

    let temp = std::env::temp_dir().join(format!("sens-asm-differential-{}", std::process::id()));
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("remove stale witness temp directory");
    }
    fs::create_dir_all(&temp).expect("create witness temp directory");

    let cases = [
        Case {
            name: "add_rax_rcx",
            form: "(add-r64-r64 rax rcx)",
            gas: "add rax, rcx",
            nasm: "add rax, rcx",
        },
        Case {
            name: "add_r8_r9",
            form: "(add-r64-r64 r8 r9)",
            gas: "add r8, r9",
            nasm: "add r8, r9",
        },
        Case {
            name: "store_rdi_disp8_rax",
            form: "(mov-mem-disp8-r64 rdi 8 rax)",
            gas: "mov QWORD PTR [rdi + 8], rax",
            nasm: "mov qword [rdi + 8], rax",
        },
        Case {
            name: "load_rax_rdi_disp8",
            form: "(mov-r64-mem-disp8 rax rdi 8)",
            gas: "mov rax, QWORD PTR [rdi + 8]",
            nasm: "mov rax, qword [rdi + 8]",
        },
        Case {
            name: "store_r12_disp8_r9",
            form: "(mov-mem-disp8-r64 r12 8 r9)",
            gas: "mov QWORD PTR [r12 + 8], r9",
            nasm: "mov qword [r12 + 8], r9",
        },
        Case {
            name: "aesenc_xmm0_xmm1",
            form: "(aesenc-xmm-xmm xmm0 xmm1)",
            gas: "aesenc xmm0, xmm1",
            nasm: "aesenc xmm0, xmm1",
        },
        Case {
            name: "aesenc_xmm8_xmm9",
            form: "(aesenc-xmm-xmm xmm8 xmm9)",
            gas: "aesenc xmm8, xmm9",
            nasm: "aesenc xmm8, xmm9",
        },
        Case {
            name: "pclmulqdq_xmm0_xmm1_11",
            form: "(pclmulqdq-xmm-xmm-imm8 xmm0 xmm1 17)",
            gas: "pclmulqdq xmm0, xmm1, 0x11",
            nasm: "pclmulqdq xmm0, xmm1, 0x11",
        },
        Case {
            name: "pclmulqdq_xmm8_xmm9_01",
            form: "(pclmulqdq-xmm-xmm-imm8 xmm8 xmm9 1)",
            gas: "pclmulqdq xmm8, xmm9, 0x01",
            nasm: "pclmulqdq xmm8, xmm9, 0x01",
        },
        Case {
            name: "ret",
            form: "(ret)",
            gas: "ret",
            nasm: "ret",
        },
    ];

    let mut session = machine_session();
    for case in cases {
        let sens = sens_bytes(case.form, &mut session);
        let gas = gas_bytes(&temp, case.name, case.gas);
        let nasm = nasm_bytes(&temp, case.name, case.nasm);

        assert_eq!(
            gas, sens,
            "{}: GAS bytes differ from Lisp-owned encoder for {}",
            case.name, case.form
        );
        assert_eq!(
            nasm, sens,
            "{}: NASM bytes differ from Lisp-owned encoder for {}",
            case.name, case.form
        );
    }

    fs::remove_dir_all(&temp).expect("clean witness temp directory");
}


// Окремий від зовнішніх асемблерів етапний свідок виклику.
// Якщо реальна збірка ще заблокована, звіт показує перший несправний вузол.
#[test]
#[ignore = "машинний GitHub-hosted маршрут; той самий профіль, що GAS/NASM"]
fn x86_encoder_callability_stage_probe() {
    let mut session = machine_session();
    let stages = [
        ("код_rax", "(x86-reg-code (quote rax))"),
        ("код_rcx", "(x86-reg-code (quote rcx))"),
        ("залишок_3", "(x86-low3 1)"),
        ("верхній_1", "(x86-high1 1)"),
        ("префікс_REX", "(x86-encode-rex 1 0 0 0)"),
        ("поле_ModRM", "(x86-encode-modrm 3 1 0)"),
        ("код_xmm0", "(x86-xmm-reg-code (quote xmm0))"),
        ("код_xmm1", "(x86-xmm-reg-code (quote xmm1))"),
        ("верхній_xmm0", "(x86-high1 (x86-xmm-reg-code (quote xmm0)))"),
        ("верхній_xmm1", "(x86-high1 (x86-xmm-reg-code (quote xmm1)))"),
        ("умова_без_REX_XMM", "(менше? (додати (x86-high1 (x86-xmm-reg-code (quote xmm0))) (x86-high1 (x86-xmm-reg-code (quote xmm1)))) 1)"),
        ("SSE_66_енкодер", "(x86-encode-sse-66-map-xmm-xmm 56 220 (quote xmm0) (quote xmm1))"),
        ("AESENC_енкодер", "(x86-encode-aesenc-xmm-xmm (quote xmm0) (quote xmm1))"),
        ("PCLMUL_exact_integer", "(x86-admission-exact-integer? 17)"),
        ("PCLMUL_range_uimm8", "(x86-admission-within-inclusive-integer-range? 17 0 255)"),
        ("PCLMUL_uimm8", "(x86-admission-uimm8? 17)"),
        ("PCLMUL_pattern_match", "(x86-admission-pattern-match? (quote (pclmulqdq-xmm-xmm-imm8 xmm-register xmm-register uimm8)) (quote (pclmulqdq-xmm-xmm-imm8 xmm0 xmm1 17)))"),
        ("PCLMUL_admitted", "(x86-admitted-instruction? (quote (pclmulqdq-xmm-xmm-imm8 xmm0 xmm1 17)))"),
        ("PCLMUL_SSE_map", "(x86-encode-sse-66-map-xmm-xmm 58 104 (quote xmm0) (quote xmm1))"),
        ("PCLMUL_encoder_direct", "(x86-encode-pclmulqdq-xmm-xmm-imm8 (quote xmm0) (quote xmm1) 17)"),
        ("PCLMUL_instruction_encoder", "(x86-encode-admitted-instruction (quote (pclmulqdq-xmm-xmm-imm8 xmm0 xmm1 17)))"),
        ("PCLMUL_whole_program", "(x86-encode-admitted-program (quote ((pclmulqdq-xmm-xmm-imm8 xmm0 xmm1 17))))"),
        ("старий_LIST", "(00100111 72 1 200)"),
        ("чинний_LIST", "(list 72 1 200)"),
        ("старий_LET", "(10011100 ((n 1)) n)"),
        ("чинний_LET", "(let ((n 1)) n)"),
        ("байти_add", "(x86-encode-add-r64-r64 (quote rax) (quote rcx))"),
        (
            "одна_інструкція",
            "(x86-encode-admitted-instruction (quote (add-r64-r64 rax rcx)))",
        ),
        (
            "повна_програма",
            "(x86-encode-admitted-program (quote ((add-r64-r64 rax rcx))))",
        ),
    ];
    let mut failed = Vec::new();
    for (stage, source) in stages {
        match eval_program(source, &mut session) {
            Ok(observation) => eprintln!(
                "X86_ЕТАП_УСПІХ stage={stage} value={}",
                observation.value
            ),
            Err(error) => {
                eprintln!("X86_ЕТАП_БЛОКУВАННЯ stage={stage} error={error}");
                failed.push(stage);
            }
        }
    }
    assert!(
        failed.is_empty(),
        "Не доведено виклики x86-кодувальника на етапах: {failed:?}"
    );
}


// Незалежна атестація всіх 16 механічних номерів регістрів.
// Перевіряємо точний результат, а не тільки відсутність помилки виклику.
// #d потрібен, щоб числа 10/11 не стали двійковими словами.
#[test]
#[ignore = "разом з GAS/NASM на GitHub-hosted runner"]
fn x86_register_fields_have_exact_values_for_all_16_codes() {
    let mut session = machine_session();
    for code in 0u32..16 {
        let low_source = format!("(x86-low3 #d{code})");
        let high_source = format!("(x86-high1 #d{code})");
        let low = eval_program(&low_source, &mut session)
            .unwrap_or_else(|error| panic!("Невикличний low3 для {code}: {error}"));
        let high = eval_program(&high_source, &mut session)
            .unwrap_or_else(|error| panic!("Невикличний high1 для {code}: {error}"));
        assert_eq!(
            low.value.to_string(),
            (code % 8).to_string(),
            "Невірні молодші три біти регістра {code}"
        );
        assert_eq!(
            high.value.to_string(),
            (code / 8).to_string(),
            "Невірний старший біт регістра {code}"
        );
        eprintln!("X86_ПОЛЯ_РЕГІСТРА_УСПІХ code={code} low3={} high1={}",
                  low.value, high.value);
    }
}


// Атестуємо саме генератор ADD, окремо від ще заблокованого admission.
// Кожне байтове очікування порівнюється також із двома незалежними асемблерами.
#[test]
#[ignore = "зовнішні GAS/NASM на GitHub-hosted runner"]
fn x86_direct_add_byte_parity_with_gas_nasm() {
    assert!(tool_version("as", &["--version"]).ends_with(" 2.42"));
    assert_eq!(tool_version("nasm", &["-v"]), "NASM version 2.16.01");
    assert!(tool_version("objcopy", &["--version"]).ends_with(" 2.42"));

    let mut session = machine_session();
    let temp = std::env::temp_dir().join(format!("sens-asm-direct-add-{}", std::process::id()));
    if temp.exists() {
        fs::remove_dir_all(&temp).expect("прибрати попередній артефакт");
    }
    fs::create_dir_all(&temp).expect("створити каталог незалежного свідка");
    let cases: [(&str, &str, &str, &str, &[u8]); 2] = [
        ("rax_rcx", "rax", "rcx", "add rax, rcx", &[72, 1, 200]),
        ("r8_r9", "r8", "r9", "add r8, r9", &[77, 1, 200]),
    ];
    for (name, destination, source, instruction, expected) in cases {
        let expression = format!(
            "(x86-encode-add-r64-r64 (quote {destination}) (quote {source}))"
        );
        let observed = eval_program(&expression, &mut session)
            .unwrap_or_else(|error| panic!("Недоступний прямий ADD {name}: {error}"));
        let bytes = parse_byte_list(&observed.value.to_string());
        assert_eq!(bytes, expected, "{name}: невірні незалежні байтові очікування");
        let gas = gas_bytes(&temp, name, instruction);
        let nasm = nasm_bytes(&temp, name, instruction);
        assert_eq!(bytes, gas, "{name}: різниця з GAS");
        assert_eq!(bytes, nasm, "{name}: різниця з NASM");
        eprintln!("X86_ПРЯМИЙ_ADD_ПАРИТЕТ_УСПІХ name={name} bytes={bytes:?}");
    }
    fs::remove_dir_all(&temp).expect("очистити фізичне свідчення");
}


// Перевірка етапу складання фізичних байтів — незалежно від admission.
// Порожній, один та два блоки: точний порядок без англійських macro-підмін.
#[test]
#[ignore = "штатний профіль x86 на GitHub-hosted runner"]
fn x86_byte_program_stitches_without_legacy_callable() {
    let mut session = machine_session();
    let cases = [
        ("порожня", "(x86-encode-program (quote ()))", "()"),
        ("одна", "(x86-encode-program (quote ((72 1 200))))", "(72 1 200)"),
        (
            "дві",
            "(x86-encode-program (quote ((72 1 200) (195))))",
            "(72 1 200 195)",
        ),
    ];
    for (name, source, expected) in cases {
        let observed = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("X86_СКЛАДАННЯ_БЛОКУВАННЯ stage={name}: {error}"));
        assert_eq!(
            observed.value.to_string(),
            expected,
            "X86_СКЛАДАННЯ_НЕСПІВПАДІННЯ stage={name}"
        );
        eprintln!("X86_СКЛАДАННЯ_УСПІХ stage={name} bytes={expected}");
    }
    // Довільний атом не є ознакою EOF; він повинен відхилятися, а не
    // матеріалізуватися у порожній машинний потік.
    let malformed = eval_program(
        "(x86-encode-program (quote не-список))",
        &mut session,
    )
    .expect("non-list instruction input must return the explicit SENS rejection envelope");
    assert!(
        malformed.value.to_string().starts_with("(rejected machine-operand instruction-list"),
        "Неспискова структура має повернути явне SENS-відхилення, а не порожній потік: {}",
        malformed.value
    );
}
