use sens::{eval_program, eval_parsed_expressions, load_core_library, parse_mixed_exact_domain_machine_source, Session};
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

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before machine block round-trip");
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

fn encode_canonical_forms(forms: &str, session: &mut Session) -> Vec<u8> {
    let encoded = eval_value(
        &format!("(x86-encode-admitted-program (00000001 {forms}))"),
        session,
    );
    parse_byte_list(&encoded)
}

fn assert_canonical_round_trip(forms: &str, session: &mut Session) {
    let bytes = encode_canonical_forms(forms, session);
    let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
        .unwrap_or_else(|error| panic!("independent decoder rejected {bytes:?}: {error}"));
    let normalized = render_forms(&decoded);

    assert_eq!(
        normalized, forms,
        "decode(encode(forms)) must reconstruct the canonical structured forms"
    );

    let reencoded = encode_canonical_forms(&normalized, session);
    assert_eq!(
        reencoded, bytes,
        "encode(decode(bytes)) must reproduce canonical admitted bytes"
    );
}

#[test]
fn composed_machine_block_round_trips_through_independent_decoder() {
    let mut session = machine_session();
    let block = eval_value(
        "(machine-block-append (machine-block-append (machine-block-append (machine-block-one (x86-mov-r64-imm64 (quote rax) 2)) (x86-mov-r64-imm64 (quote rcx) 3)) (x86-add-r64-r64 (quote rax) (quote rcx))) (x86-ret))",
        &mut session,
    );
    let encoded = eval_value(
        &format!("(x86-encode-machine-block (quote {block}))"),
        &mut session,
    );
    let bytes = parse_byte_list(&encoded);

    let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
        .unwrap_or_else(|error| panic!("independent decoder rejected {bytes:?}: {error}"));

    assert_eq!(
        render_forms(&decoded),
        block,
        "decoder must reconstruct the exact Lisp-owned machine forms, not merely accept the byte stream"
    );
}

#[test]
fn current_observer_subset_has_bidirectional_canonical_round_trip() {
    let mut session = machine_session();

    let cases = [
        "((ret))",
        "((mov-r64-imm64 rax 0))",
        "((mov-r64-imm64 rsp 42))",
        "((mov-r64-imm64 r8 4294967295))",
        "((mov-r64-imm64 r15 18446744073709551615))",
        "((add-r64-r64 rax rcx))",
        "((or-r64-r64 r8 r15))",
        "((and-r64-r64 rsp r12))",
        "((sub-r64-r64 r15 rax))",
        "((xor-r64-r64 r9 r10))",
        "((cmp-r64-r64 r12 rsp))",
        "((mov-mem-disp8-r64 rax -128 rdx))",
        "((mov-mem-disp8-r64 rsp -1 r8))",
        "((mov-mem-disp8-r64 r12 0 r15))",
        "((mov-mem-disp8-r64 r15 127 rax))",
        "((mov-r64-mem-disp8 rdx rax -128))",
        "((mov-r64-mem-disp8 r8 rsp -1))",
        "((mov-r64-mem-disp8 r15 r12 0))",
        "((mov-r64-mem-disp8 rax r15 127))",
        "((jnz-rel8 -128))",
        "((jnz-rel8 -1))",
        "((jnz-rel8 0))",
        "((jnz-rel8 1))",
        "((jnz-rel8 127))",
        "((mov-r64-imm64 r8 2) (mov-r64-imm64 r9 3) (add-r64-r64 r8 r9) (ret))",
        "((mov-r64-imm64 r10 7) (mov-mem-disp8-r64 r12 8 r10) (mov-r64-mem-disp8 r15 r12 8) (ret))",
    ];

    for forms in cases {
        assert_canonical_round_trip(forms, &mut session);
    }
}

#[test]
fn independent_decoder_fails_closed_on_truncated_machine_block() {
    let mut session = machine_session();
    let encoded = eval_value(
        "(x86-encode-machine-block (machine-block-one (x86-mov-r64-imm64 (quote rax) 42)))",
        &mut session,
    );
    let mut bytes = parse_byte_list(&encoded);
    bytes.pop();

    assert!(
        x86_64_block_decoder::decode_machine_block(&bytes).is_err(),
        "truncated machine bytes must not be normalized into a different valid form"
    );
}
