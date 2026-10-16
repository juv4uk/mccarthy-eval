use sens::{eval_parsed_expressions, eval_program, load_core_library, parse_mixed_exact_domain_machine_source, Session};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn encoder_session() -> Session {
    let path = repo_root().join("lib/machine/encoding/x86-64.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));

    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before machine encoder");
    let expressions = parse_mixed_exact_domain_machine_source("lib/machine/encoding/x86-64.lisp", &source)
        .expect("x86-64 encoder must parse through the exact-domain machine-source reader");
    eval_parsed_expressions(&expressions, &mut session)
        .expect("x86-64 encoder must load through the exact-domain machine-source reader");
    session
}

fn eval_bytes(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("encoder expression failed: {source}: {error}"))
        .value
        .to_string()
}

fn admitted_encoder_session() -> Session {
    let mut session = encoder_session();
    let path = repo_root().join("lib/machine/admission/x86-64.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
    let expressions = parse_mixed_exact_domain_machine_source("lib/machine/admission/x86-64.lisp", &source)
        .expect("x86-64 admission must parse through the exact-domain machine-source reader");
    eval_parsed_expressions(&expressions, &mut session)
        .expect("x86-64 admission must load through the exact-domain machine-source reader");
    session
}


#[test]
fn binary_u3_machine_field_projection_is_explicit_total_and_fail_closed() {
    let mut session = encoder_session();
    for (source, expected) in [
        ("#b0", "0"),
        ("#b1", "1"),
        ("#b10", "2"),
        ("#b11", "3"),
        ("#b100", "4"),
        ("#b101", "5"),
        ("#b110", "6"),
        ("#b111", "7"),
    ] {
        let form = format!("(x86-project-bin3-to-mechanism-u3 {source})");
        assert_eq!(eval_bytes(&form, &mut session), expected, "form: {form}");
    }

    assert_eq!(
        eval_bytes("(x86-project-bin3-to-mechanism-u3 #b1000)", &mut session),
        "()",
        "u3 projection must fail closed outside 0..7"
    );
}

#[test]
fn admitted_rdrand_rdseed_programs_materialize_expected_bytes_through_binary_u3_bridge() {
    let mut session = admitted_encoder_session();

    assert_eq!(
        eval_bytes(
            "(x86-encode-admitted-program (quote ((rdrand-r64 rax) (ret))))",
            &mut session
        ),
        "(72 15 199 240 195)"
    );
    assert_eq!(
        eval_bytes(
            "(x86-encode-admitted-program (quote ((rdrand-r64 r8) (ret))))",
            &mut session
        ),
        "(73 15 199 240 195)"
    );
    assert_eq!(
        eval_bytes(
            "(x86-encode-admitted-program (quote ((rdseed-r64 rax) (ret))))",
            &mut session
        ),
        "(72 15 199 248 195)"
    );
    assert_eq!(
        eval_bytes(
            "(x86-encode-admitted-program (quote ((rdseed-r64 r15) (ret))))",
            &mut session
        ),
        "(73 15 199 255 195)"
    );
}

#[test]
fn register_ordinals_10_and_11_preserve_decimal_values() {
    let mut session = encoder_session();
    for (form, expected) in [
        ("(x86-reg-code (quote r10))", "10"),
        ("(x86-reg-code (quote r10b))", "10"),
        ("(x86-reg-code (quote r11))", "11"),
        ("(x86-reg-code (quote r11b))", "11"),
        ("(x86-xmm-reg-code (quote xmm10))", "10"),
        ("(x86-xmm-reg-code (quote xmm11))", "11"),
    ] {
        assert_eq!(eval_bytes(form, &mut session), expected, "form: {form}");
    }
}

#[test]
fn parity_condition_wrappers_preserve_decimal_codes_10_and_11() {
    let mut session = encoder_session();
    for (wrapper, generic) in [
        ("(x86-encode-jp-rel8 0)", "(x86-encode-jcc-rel8 #d10 0)"),
        ("(x86-encode-jnp-rel8 0)", "(x86-encode-jcc-rel8 #d11 0)"),
        ("(x86-encode-jp-rel32 0)", "(x86-encode-jcc-rel32 #d10 0)"),
        ("(x86-encode-jnp-rel32 0)", "(x86-encode-jcc-rel32 #d11 0)"),
        ("(x86-encode-setp-r8 (quote al))", "(x86-encode-setcc-r8 #d10 (quote al))"),
        ("(x86-encode-setnp-r8 (quote al))", "(x86-encode-setcc-r8 #d11 (quote al))"),
        ("(x86-encode-cmovp-r64-r64 (quote rax) (quote rbx))", "(x86-encode-cmovcc-r64-r64 #d10 (quote rax) (quote rbx))"),
        ("(x86-encode-cmovnp-r64-r64 (quote rax) (quote rbx))", "(x86-encode-cmovcc-r64-r64 #d11 (quote rax) (quote rbx))"),
    ] {
        assert_eq!(
            eval_bytes(wrapper, &mut session),
            eval_bytes(generic, &mut session),
            "condition wrapper must preserve its explicit decimal condition code: {wrapper}",
        );
    }
}

#[test]
fn movq_xmm_r64_keeps_opcode_byte_0x6e_as_decimal_110() {
    let mut session = encoder_session();
    let rendered = eval_bytes(
        "(x86-encode-movq-xmm-r64 (quote xmm0) (quote rax))",
        &mut session,
    );
    let bytes = rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .map(|token| token.parse::<u8>().expect("MOVQ byte must stay in u8 range"))
        .collect::<Vec<_>>();
    assert_eq!(bytes, vec![102, 72, 15, 110, 192]);
    assert_eq!(bytes[3], 110, "0x6E opcode must not become binary 110 = decimal 6");
}

#[test]
fn lisp_encodes_ret_to_exact_machine_byte() {
    let mut session = encoder_session();
    assert_eq!(eval_bytes("(x86-encode-ret)", &mut session), "(195)");
}

#[test]
fn lisp_encodes_mov_eax_imm32_little_endian() {
    let mut session = encoder_session();
    for (source, expected) in [
        ("(x86-encode-mov-eax-imm32 0)", "(184 0 0 0 0)"),
        ("(x86-encode-mov-eax-imm32 42)", "(184 42 0 0 0)"),
        ("(x86-encode-mov-eax-imm32 305419896)", "(184 120 86 52 18)"),
        ("(x86-encode-mov-eax-imm32 4294967295)", "(184 255 255 255 255)"),
    ] {
        assert_eq!(eval_bytes(source, &mut session), expected, "source: {source}");
    }
}

#[test]
fn lisp_encodes_add_rax_rbx_without_external_assembler() {
    let mut session = encoder_session();
    assert_eq!(
        eval_bytes(
            "(x86-encode-add-r64-r64 (quote rax) (quote rbx))",
            &mut session
        ),
        "(72 1 216)"
    );
}

/// #176 TDD requirement: compare emitted bytes against pinned external
/// evidence. lib/machine/xed/vendor/base/xed-isa.txt (pinned at #175's
/// commit) is upstream Intel XED's own encoding for the no-operand
/// RET_NEAR form: `PATTERN : 0xC3 ...`. This proves the Lisp encoder's
/// opcode choice for `(ret)` was not invented independently of the
/// admitted ISA evidence it claims to cover.
#[test]
fn ret_encoding_matches_pinned_xed_pattern_for_ret_near() {
    let mut session = encoder_session();
    let emitted = eval_bytes("(x86-encode-ret)", &mut session);
    assert_eq!(emitted, "(195)", "195 decimal must equal 0xC3");

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));

    // The no-operand near-return block: ICLASS RET_NEAR whose PATTERN has
    // no OPERANDS-affecting immediate (the imm16 stack-adjust variant is a
    // separate block starting `PATTERN : 0xC2 ...` and is not what
    // `x86-encode-ret` claims to cover).
    assert!(
        vendor_source.contains("PATTERN   : 0xC3 DF64() IMMUNE66_LOOP64()"),
        "pinned XED evidence must still contain the exact RET_NEAR/0xC3 pattern this encoder was checked against"
    );
}

/// #176 continued: the group-1 ALU register/register family (OR/AND/SUB/
/// XOR/CMP) sharing ADD's shape, one opcode byte apart, per Intel's
/// canonical group-1 opcode layout confirmed against #175's pinned XED
/// evidence for each ICLASS's `MOD[0b11] MOD=3 REG[rrr] RM[nnn]` form.
#[test]
fn lisp_encodes_the_alu_register_family_with_pinned_opcodes() {
    let mut session = encoder_session();
    for (form, expected, xed_pattern) in [
        (
            "(x86-encode-or-r64-r64 (quote rax) (quote rbx))",
            "(72 9 216)",
            "PATTERN   : 0x09 MOD[0b11] MOD=3 REG[rrr] RM[nnn]",
        ),
        (
            "(x86-encode-and-r64-r64 (quote rax) (quote rbx))",
            "(72 33 216)",
            "PATTERN   : 0x21 MOD[0b11] MOD=3 REG[rrr] RM[nnn]",
        ),
        (
            "(x86-encode-sub-r64-r64 (quote rax) (quote rbx))",
            "(72 41 216)",
            "PATTERN   : 0x29 MOD[0b11] MOD=3 REG[rrr] RM[nnn]",
        ),
        (
            "(x86-encode-xor-r64-r64 (quote rax) (quote rbx))",
            "(72 49 216)",
            "PATTERN   : 0x31 MOD[0b11] MOD=3 REG[rrr] RM[nnn]",
        ),
        (
            "(x86-encode-cmp-r64-r64 (quote rax) (quote rbx))",
            "(72 57 216)",
            "PATTERN   : 0x39 MOD[0b11] MOD=3 REG[rrr] RM[nnn]",
        ),
    ] {
        assert_eq!(eval_bytes(form, &mut session), expected, "form: {form}");

        let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
        let vendor_source = fs::read_to_string(&vendor_path)
            .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
        assert!(
            vendor_source.contains(xed_pattern),
            "pinned XED evidence must contain the exact pattern {form} was checked against: {xed_pattern}"
        );
    }
}

/// Independent decoder for exactly the PUSH r64 / POP r64 byte shapes this
/// encoder emits: an optional REX prefix (0x40-0x4F, bit 0 = REX.B) followed
/// by a single opcode byte in 0x50-0x5F. This is deliberately a *second*,
/// from-scratch implementation of the bit arithmetic (decode direction, not
/// mirroring the encoder's own construction), so a test built only from the
/// encoder's own math could not pass it by coincidence.
fn decode_push_or_pop(bytes: &[u8]) -> Option<(&'static str, u8)> {
    let (rex_b, rest) = match bytes {
        [rex, rest @ ..] if (0x40..=0x4F).contains(rex) => ((rex & 0x01) != 0, rest),
        rest => (false, rest),
    };
    let [opcode] = rest else { return None };
    let reg_low3 = opcode & 0b0000_0111;
    let reg = reg_low3 | if rex_b { 0b1000 } else { 0 };
    match opcode & 0b1111_1000 {
        0x50 => Some(("push", reg)),
        0x58 => Some(("pop", reg)),
        _ => None,
    }
}

/// #176 continued: PUSH r64 (opcode 0x50+rd) / POP r64 (opcode 0x58+rd),
/// both `DF64()` (default 64-bit operand size in long mode, no REX.W)
/// per #175's pinned XED evidence, with REX.B only for r8-r15. Verified two
/// ways: against the pinned XED PATTERN text, and by independently decoding
/// the emitted bytes back to (mnemonic, register index).
#[test]
fn lisp_encodes_push_and_pop_with_pinned_opcodes_and_independent_decode() {
    let mut session = encoder_session();
    let registers = [
        ("rax", 0u8),
        ("rcx", 1),
        ("rdx", 2),
        ("rbx", 3),
        ("rsp", 4),
        ("rbp", 5),
        ("rsi", 6),
        ("rdi", 7),
        ("r8", 8),
        ("r9", 9),
        ("r10", 10),
        ("r11", 11),
        ("r12", 12),
        ("r13", 13),
        ("r14", 14),
        ("r15", 15),
    ];

    for (register_name, register_code) in registers {
        for (op, mnemonic) in [("push", "push"), ("pop", "pop")] {
            let form = format!("(x86-encode-{op}-r64 (quote {register_name}))");
            let rendered = eval_bytes(&form, &mut session);
            let bytes: Vec<u8> = rendered
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(|token| token.parse().expect("byte must be a small integer"))
                .collect();

            let decoded = decode_push_or_pop(&bytes)
                .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
            assert_eq!(
                decoded,
                (mnemonic, register_code),
                "{form} round-tripped to {decoded:?} via independent decode, from bytes {bytes:?}"
            );

            if register_code < 8 {
                assert_eq!(bytes.len(), 1, "{form} for a low register must need no REX prefix");
            } else {
                assert_eq!(bytes.len(), 2, "{form} for r8-r15 must carry REX.B");
                assert_eq!(bytes[0], 0x41, "REX.B-only prefix must be exactly 0x41");
            }
        }
    }

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
    assert!(
        vendor_source.contains("PATTERN   : 0b0101_0 SRM[rrr] REX2=0 DF64()"),
        "pinned XED evidence must contain the exact PUSH r64 pattern this encoder was checked against"
    );
    assert!(
        vendor_source.contains("PATTERN   : 0b0101_1 SRM[rrr] REX2=0 DF64()"),
        "pinned XED evidence must contain the exact POP r64 pattern this encoder was checked against"
    );
}

/// Independent decoder for the INC r64 / DEC r64 group-5 shape this encoder
/// emits: REX.W (0x48 or 0x49) + opcode 0xFF + ModRM(mod=3, reg, rm). `reg`
/// selects INC (0) vs DEC (1) -- it is an opcode extension, not a register
/// operand. A from-scratch decode, independent of the encoder's own
/// construction arithmetic.
fn decode_inc_or_dec(bytes: &[u8]) -> Option<(&'static str, u8)> {
    let [rex, opcode, modrm] = bytes else { return None };
    if *opcode != 0xFF {
        return None;
    }
    let rex_w = (rex & 0x08) != 0;
    let rex_b = (rex & 0x01) != 0;
    if !rex_w || (rex & 0xF0) != 0x40 {
        return None;
    }
    let mod_bits = modrm >> 6;
    let reg_field = (modrm >> 3) & 0b111;
    let rm_field = modrm & 0b111;
    if mod_bits != 0b11 {
        return None;
    }
    let register = rm_field | if rex_b { 0b1000 } else { 0 };
    match reg_field {
        0 => Some(("inc", register)),
        1 => Some(("dec", register)),
        _ => None,
    }
}

/// #176 continued: INC r64 / DEC r64 always go through the group-5 ModRM
/// path (opcode 0xFF, /0 or /1), never the legacy single-byte 0x40+r/
/// 0x48+r form -- that pinned XED evidence itself tags the legacy form
/// `not64`, since those exact byte values are REX prefixes in 64-bit mode.
/// Getting this wrong (emitting the legacy form) would silently corrupt
/// any following instruction's REX prefix, so this is exactly the kind of
/// "illegal encoding for this mode" case the encoder must never produce.
#[test]
fn lisp_encodes_inc_and_dec_via_group5_modrm_never_the_not64_legacy_form() {
    let mut session = encoder_session();
    for (register_name, register_code) in [("rax", 0u8), ("rcx", 1), ("r8", 8), ("r15", 15)] {
        for (op, mnemonic) in [("inc", "inc"), ("dec", "dec")] {
            let form = format!("(x86-encode-{op}-r64 (quote {register_name}))");
            let rendered = eval_bytes(&form, &mut session);
            let bytes: Vec<u8> = rendered
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(|token| token.parse().expect("byte must be a small integer"))
                .collect();

            assert_eq!(bytes.len(), 3, "{form} must always be REX+0xFF+ModRM, 3 bytes");
            assert_eq!(bytes[1], 255, "{form} must use group-5 opcode 0xFF, never the not64 legacy form");

            let decoded = decode_inc_or_dec(&bytes)
                .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
            assert_eq!(
                decoded,
                (mnemonic, register_code),
                "{form} round-tripped to {decoded:?} via independent decode, from bytes {bytes:?}"
            );
        }
    }

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
    assert!(
        vendor_source.contains("PATTERN   : 0xFF MOD[0b11] MOD=3 REG[0b000] RM[nnn]"),
        "pinned XED evidence must contain the exact INC group-5 pattern this encoder was checked against"
    );
    assert!(
        vendor_source.contains("PATTERN   : 0xFF MOD[0b11] MOD=3 REG[0b001] RM[nnn]"),
        "pinned XED evidence must contain the exact DEC group-5 pattern this encoder was checked against"
    );
    assert!(
        vendor_source.contains("PATTERN   : 0b0100_0 SRM[rrr] not64"),
        "pinned XED evidence must still tag the legacy single-byte INC form not64 -- if this ever changes, the group-5-only encoding choice needs re-justifying"
    );
}

/// Independent decoder for the NOT r64 / NEG r64 group-3 shape: REX.W
/// (0x48 or 0x49) + opcode 0xF7 + ModRM(mod=3, reg, rm). `reg` selects NOT
/// (2) vs NEG (3), an opcode extension, not a register operand. From
/// scratch, independent of the encoder's own construction arithmetic.
fn decode_not_or_neg(bytes: &[u8]) -> Option<(&'static str, u8)> {
    let [rex, opcode, modrm] = bytes else { return None };
    if *opcode != 0xF7 {
        return None;
    }
    let rex_w = (rex & 0x08) != 0;
    let rex_b = (rex & 0x01) != 0;
    if !rex_w || (rex & 0xF0) != 0x40 {
        return None;
    }
    let mod_bits = modrm >> 6;
    let reg_field = (modrm >> 3) & 0b111;
    let rm_field = modrm & 0b111;
    if mod_bits != 0b11 {
        return None;
    }
    let register = rm_field | if rex_b { 0b1000 } else { 0 };
    match reg_field {
        2 => Some(("not", register)),
        3 => Some(("neg", register)),
        _ => None,
    }
}

/// #176 continued: NOT r64 / NEG r64 always go through group-3 (opcode
/// 0xF7, /2 or /3), the same shape as INC/DEC's group-5 (opcode 0xFF, /0
/// or /1) -- a distinct opcode byte and reg-field pair, not something
/// this decoder can confuse with INC/DEC by construction (a differing
/// opcode byte fails the match outright).
#[test]
fn lisp_encodes_not_and_neg_via_group3_modrm_with_independent_decode() {
    let mut session = encoder_session();
    for (register_name, register_code) in [("rax", 0u8), ("rdx", 2), ("r9", 9), ("r14", 14)] {
        for (op, mnemonic) in [("not", "not"), ("neg", "neg")] {
            let form = format!("(x86-encode-{op}-r64 (quote {register_name}))");
            let rendered = eval_bytes(&form, &mut session);
            let bytes: Vec<u8> = rendered
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(|token| token.parse().expect("byte must be a small integer"))
                .collect();

            assert_eq!(bytes.len(), 3, "{form} must always be REX+0xF7+ModRM, 3 bytes");
            assert_eq!(bytes[1], 247, "{form} must use group-3 opcode 0xF7");

            let decoded = decode_not_or_neg(&bytes)
                .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
            assert_eq!(
                decoded,
                (mnemonic, register_code),
                "{form} round-tripped to {decoded:?} via independent decode, from bytes {bytes:?}"
            );
        }
    }

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
    assert!(
        vendor_source.contains("PATTERN   : 0xF7 MOD[0b11] MOD=3 REG[0b010] RM[nnn]"),
        "pinned XED evidence must contain the exact NOT group-3 pattern this encoder was checked against"
    );
    assert!(
        vendor_source.contains("PATTERN   : 0xF7 MOD[0b11] MOD=3 REG[0b011] RM[nnn]"),
        "pinned XED evidence must contain the exact NEG group-3 pattern this encoder was checked against"
    );
}

/// #176 continued: TEST r/m64, r64 (opcode 0x85 /r) shares the group-1 ALU
/// family's exact REX.W+opcode+ModRM shape, confirmed against #175's
/// pinned XED evidence for TEST's own `MOD[0b11] MOD=3 REG[rrr] RM[nnn]`
/// form -- reusing x86-encode-alu-r64-r64 was a deliberate choice, not an
/// assumption that any group-1-shaped opcode automatically belongs there.
#[test]
fn lisp_encodes_test_r64_r64_matching_pinned_opcode() {
    let mut session = encoder_session();
    assert_eq!(
        eval_bytes(
            "(x86-encode-test-r64-r64 (quote rax) (quote rbx))",
            &mut session
        ),
        "(72 133 216)",
        "133 decimal must equal 0x85"
    );

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
    assert!(
        vendor_source.contains("PATTERN   : 0x85 MOD[0b11] MOD=3 REG[rrr] RM[nnn]"),
        "pinned XED evidence must contain the exact TEST pattern this encoder was checked against"
    );
}

/// #176 continued: the `mov-r64-mem-disp8` / `mov-mem-disp8-r64` encoder
/// was already structurally general (arbitrary GPR base/destination, SIB
/// for rsp/r12, REX.B for r8-r15) but had a real latent bug for negative
/// displacements: `(mod displacement 256)` does not wrap negative Lisp
/// numbers (`(mod -1 256)` is -1, not 255), so any negative disp8 produced
/// an out-of-range "byte" that `native-call-u64-raw` would reject at the
/// host boundary -- fail-closed, but it meant the encoder's claimed
/// generality was never actually true for negative offsets. This is a RED
/// witness for that bug plus a round-trip proof it's fixed.
fn decode_mov_mem_disp8(bytes: &[u8]) -> Option<(&'static str, u8, u8, i32)> {
    let mut i = 0;
    let rex = *bytes.get(i)?;
    if (rex & 0xF0) != 0x40 || (rex & 0x08) == 0 {
        return None;
    }
    i += 1;
    let opcode = *bytes.get(i)?;
    let direction = match opcode {
        0x8B => "load", // MOV r64, [mem]
        0x89 => "store", // MOV [mem], r64
        _ => return None,
    };
    i += 1;
    let modrm = *bytes.get(i)?;
    i += 1;
    if (modrm >> 6) != 0b01 {
        return None;
    }
    let reg_field = (modrm >> 3) & 0b111;
    let rm_field = modrm & 0b111;
    let rex_r = (rex & 0x04) != 0;
    let rex_b = (rex & 0x01) != 0;
    let reg = reg_field | if rex_r { 0b1000 } else { 0 };
    let base = if rm_field == 0b100 {
        // SIB byte present for rsp/r12 bases.
        let sib = *bytes.get(i)?;
        i += 1;
        if sib != 0x24 {
            return None; // scale=0, index=none (0b100) only
        }
        0b100 | if rex_b { 0b1000 } else { 0 }
    } else {
        rm_field | if rex_b { 0b1000 } else { 0 }
    };
    let disp_byte = *bytes.get(i)?;
    if bytes.len() != i + 1 {
        return None;
    }
    let disp = disp_byte as i8 as i32;
    Some((direction, reg, base, disp))
}

fn gpr_index(name: &str) -> u8 {
    [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15",
    ]
    .iter()
    .position(|candidate| *candidate == name)
    .expect("known GPR name") as u8
}

#[test]
fn lisp_encodes_mov_disp8_for_every_gpr_base_with_correct_negative_displacement() {
    const ALL_GPRS: [&str; 16] = [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15",
    ];

    let mut session = encoder_session();
    // Full 16 (data register, exercises REX.R) x 16 (base register,
    // exercises REX.B and SIB for rsp/r12) matrix, crossed with the disp8
    // boundary (min, -1, 0, mid, max) -- 2560 combinations total. An
    // earlier version of this witness only ever used rax as the data
    // register, which meant REX.R was never exercised at all despite
    // admission opening (mov-*-mem-disp8 register disp8 register) for any
    // of the 16 GPRs in both slots; independently cross-checked with
    // objdump against this same matrix (2560 decoded, 0 mismatches).
    for data_register in ALL_GPRS {
        for base in ALL_GPRS {
            for displacement in [-128i32, -1, 0, 1, 127] {
                let load_form = format!(
                    "(x86-encode-mov-r64-mem-disp8 (quote {data_register}) (quote {base}) {displacement})"
                );
                let store_form = format!(
                    "(x86-encode-mov-mem-disp8-r64 (quote {base}) {displacement} (quote {data_register}))"
                );

                for (form, expected_direction) in [(&load_form, "load"), (&store_form, "store")] {
                    let rendered = eval_bytes(form, &mut session);
                    let bytes: Vec<u8> = rendered
                        .trim_start_matches('(')
                        .trim_end_matches(')')
                        .split_whitespace()
                        .map(|token| token.parse().expect("byte must be a small integer"))
                        .collect();
                    // Parsing each token as u8 above is itself the RED
                    // witness for the original bug: before the fix, a
                    // negative displacement made the encoder emit a
                    // literal negative number, which would fail to parse
                    // as u8 right here.

                    let (direction, reg, base_code, decoded_disp) = decode_mov_mem_disp8(&bytes)
                        .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
                    assert_eq!(direction, expected_direction, "{form}: {bytes:?}");
                    assert_eq!(reg, gpr_index(data_register), "{form}: {bytes:?}");
                    assert_eq!(base_code, gpr_index(base), "{form}: {bytes:?}");
                    assert_eq!(
                        decoded_disp, displacement,
                        "{form} round-tripped to disp {decoded_disp}, from bytes {bytes:?}"
                    );
                }
            }
        }
    }
}

/// Independent decoder for the Jcc rel8 family: a single opcode byte in
/// 0x70-0x7F (low nibble is the condition code, no REX prefix -- this is
/// address-size/register-independent control transfer) followed by one
/// signed relative-displacement byte. From scratch, independent of the
/// encoder's own construction arithmetic.
fn decode_jcc_rel8(bytes: &[u8]) -> Option<(u8, i32)> {
    let [opcode, disp] = bytes else { return None };
    if (opcode & 0xF0) != 0x70 {
        return None;
    }
    let condition_code = opcode & 0x0F;
    let displacement = *disp as i8 as i32;
    Some((condition_code, displacement))
}

/// #176 continued: the 16-condition Jcc rel8 family (opcode 0x70+cc) --
/// the control-transfer half of the decision effect TEST/CMP already
/// provide. Per #175's pinned XED evidence, the ICLASS-to-opcode-offset
/// order is JO,JNO,JB,JNB,JZ,JNZ,JBE,JNBE,JS,JNS,JP,JNP,JL,JNL,JLE,JNLE;
/// each is verified against its own condition-code offset and the full
/// disp8 boundary, reusing x86-disp8-byte's already-proven two's-
/// complement conversion. Independently cross-checked with ndisasm (which
/// prints some of these under synonym mnemonics -- jc/jb, jnc/jnb, jna/
/// jbe, jpe/jp are the same opcode, not a discrepancy).
#[test]
fn lisp_encodes_the_full_jcc_rel8_family_with_correct_condition_codes_and_displacement() {
    let mut session = encoder_session();
    let mnemonics = [
        ("jo", 0u8),
        ("jno", 1),
        ("jb", 2),
        ("jnb", 3),
        ("jz", 4),
        ("jnz", 5),
        ("jbe", 6),
        ("jnbe", 7),
        ("js", 8),
        ("jns", 9),
        ("jp", 10),
        ("jnp", 11),
        ("jl", 12),
        ("jnl", 13),
        ("jle", 14),
        ("jnle", 15),
    ];

    for (mnemonic, expected_cc) in mnemonics {
        for displacement in [-128i32, -1, 0, 1, 127] {
            let form = format!("(x86-encode-{mnemonic}-rel8 {displacement})");
            let rendered = eval_bytes(&form, &mut session);
            let bytes: Vec<u8> = rendered
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(|token| token.parse().expect("byte must be a small integer"))
                .collect();

            assert_eq!(bytes.len(), 2, "{form} must always be opcode+disp8, 2 bytes");
            let (condition_code, decoded_disp) = decode_jcc_rel8(&bytes)
                .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
            assert_eq!(condition_code, expected_cc, "{form}: {bytes:?}");
            assert_eq!(
                decoded_disp, displacement,
                "{form} round-tripped to disp {decoded_disp}, from bytes {bytes:?}"
            );
        }
    }

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
    for iclass in [
        "JO", "JNO", "JB", "JNB", "JZ", "JNZ", "JBE", "JNBE", "JS", "JNS", "JP", "JNP", "JL",
        "JNL", "JLE", "JNLE",
    ] {
        assert!(
            vendor_source.contains(&format!("ICLASS    : {iclass}")),
            "pinned XED evidence must name ICLASS {iclass}"
        );
    }
}

/// Independent decoder for MOV r64,imm64: REX.W (+REX.B for r8-r15) followed
/// by opcode 0xB8+r (register in the low 3 opcode bits) and an 8-byte
/// little-endian immediate. From scratch, independent of the encoder's own
/// construction arithmetic.
fn decode_mov_r64_imm64(bytes: &[u8]) -> Option<(u8, u64)> {
    let [rex, opcode, imm @ ..] = bytes else {
        return None;
    };
    if rex & 0xF8 != 0x48 {
        return None;
    }
    let rex_b = rex & 0x01;
    if (*opcode & 0xF8) != 0xB8 {
        return None;
    }
    let register = (*opcode & 0x07) | (rex_b << 3);
    if imm.len() != 8 {
        return None;
    }
    let mut value = 0u64;
    for (index, byte) in imm.iter().enumerate() {
        value |= (*byte as u64) << (8 * index);
    }
    Some((register, value))
}

#[test]
fn exact_ukrainian_quotient_and_remainder_stage_probe() {
    // #5383: D5 QUOTIENT є ТОЧНИМ раціональним діленням.
    // 257/256 — не цілочисельна частка 1. Не маскувати це
    // округленням або приводити до старого SID8.
    let mut session = encoder_session();
    for (source, expected) in [
        ("(частка 0 256)", "0"),
        ("(частка 257 256)", "257/256"),
        ("(частка 65535 256)", "65535/256"),
        ("(частка 4294967296 256)", "16777216"),
    ] {
        assert_eq!(
            eval_bytes(source, &mut session),
            expected,
            "зберегти точну раціональну арифметику D5: {source}"
        );
    }

    // Від'ємний поточний свідок: MOD/остача повинна видати
    // арифметичне значення, а не помилку Type. НЕ робимо
    // очікувану помилку дозволеною поведінкою — CI має
    // залишитися червоним до справжнього виправлення.
    for (source, expected) in [
        ("(остача 0 256)", "0"),
        ("(остача 257 256)", "1"),
        ("(остача 65535 256)", "255"),
    ] {
        assert_eq!(
            eval_bytes(source, &mut session),
            expected,
            "відсутня виконувана цілочисельна остача: {source}"
        );
    }
}

#[test]
fn x86_u64_bytes_equal_independent_little_endian_for_boundaries() {
    // Незалежний байтовий оракул: стандартне Rust u64::to_le_bytes.
    // Не повторюємо алгоритм довгого ділення чи його межі.
    let mut session = encoder_session();
    for value in [
        0u64,
        1,
        255,
        256,
        257,
        65_535,
        4_294_967_295,
        4_294_967_296,
        9_007_199_254_740_991,
        u64::MAX,
    ] {
        let form = format!("(x86-u64-bytes {value})");
        let rendered = eval_bytes(&form, &mut session);
        let bytes: Vec<u8> = rendered
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split_whitespace()
            .map(|item| item.parse::<u8>().expect("лише фізичний байт 0..255"))
            .collect();
        assert_eq!(
            bytes,
            value.to_le_bytes().to_vec(),
            "x86: точні вісім little-endian байтів для {value}: {form}"
        );
    }
}

#[test]
fn x86_mov_imm64_component_stage_probe() {
    let mut session = encoder_session();
    // Відокремлюємо точну причину MOV imm64 до фізичного пакування.
    // Не змінюємо семантику та не маскуємо відмову решти регістрів.
    for (source, expected) in [
        ("(x86-reg-code (quote rax))", "0"),
        ("(x86-low3 0)", "0"),
        ("(x86-high1 0)", "0"),
        ("(x86-encode-rex 1 0 0 0)", "72"),
        ("(x86-u64-bytes 0)", "(0 0 0 0 0 0 0 0)"),
    ] {
        assert_eq!(
            eval_bytes(source, &mut session),
            expected,
            "етап фізичного MOV imm64: {source}"
        );
    }
}

/// #176 continued: generalizes MOV r64,imm64 admission from the previous
/// rax/rcx-only pair to all 16 GPRs. The encoder (`x86-encode-mov-r64-imm64`)
/// was already fully general -- REX.B for r8-r15 via the same x86-high1/
/// x86-reg-code machinery #199's disp8 work proved correct -- so this is an
/// admission-only generalization, not a new encoder, matching #199's own
/// "generalize admission around the encoder you already have" discipline.
#[test]
fn lisp_encodes_mov_r64_imm64_for_every_gpr_with_independent_decode() {
    const ALL_GPRS: [&str; 16] = [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15",
    ];
    let mut session = encoder_session();

    for register in ALL_GPRS {
        for immediate in [0u64, 1, 42, 999, 4294967296, 9007199254740991] {
            let form = format!("(x86-encode-mov-r64-imm64 (quote {register}) {immediate})");
            let rendered = eval_bytes(&form, &mut session);
            let bytes: Vec<u8> = rendered
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(|token| token.parse().expect("byte must be a small integer"))
                .collect();

            assert_eq!(bytes.len(), 10, "{form} must be REX+opcode+imm64, 10 bytes");
            let (decoded_register, decoded_immediate) = decode_mov_r64_imm64(&bytes)
                .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
            assert_eq!(decoded_register, gpr_index(register), "{form}: {bytes:?}");
            assert_eq!(
                decoded_immediate, immediate,
                "{form} round-tripped to {decoded_immediate}, from bytes {bytes:?}"
            );
        }
    }
}

/// Independent decoder for JMP rel8: a single fixed opcode byte (0xEB), no
/// condition code, no REX, no ModRM, followed by one signed relative-
/// displacement byte. From scratch, independent of the encoder's own
/// construction arithmetic.
fn decode_jmp_rel8(bytes: &[u8]) -> Option<i32> {
    let [opcode, disp] = bytes else { return None };
    if *opcode != 0xEB {
        return None;
    }
    Some(*disp as i8 as i32)
}

/// #176 continued: JMP rel8 (opcode 0xEB) -- the unconditional counterpart
/// to the Jcc rel8 family, needed as soon as a control-transfer decision
/// (e.g. a multi-clause COND, or a loop back-edge) requires more than a
/// single conditional skip. Per #175's pinned XED evidence
/// (`PATTERN : 0xEB mode64 norex2_prefix FORCE64() BRDISP8()`), reuses the
/// same x86-disp8-byte two's-complement conversion #199's MOV disp8 and
/// #202's Jcc rel8 already proved correct.
#[test]
fn lisp_encodes_jmp_rel8_with_the_pinned_opcode_and_correct_displacement() {
    let mut session = encoder_session();

    for displacement in [-128i32, -1, 0, 1, 127] {
        let form = format!("(x86-encode-jmp-rel8 {displacement})");
        let rendered = eval_bytes(&form, &mut session);
        let bytes: Vec<u8> = rendered
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split_whitespace()
            .map(|token| token.parse().expect("byte must be a small integer"))
            .collect();

        assert_eq!(bytes.len(), 2, "{form} must always be opcode+disp8, 2 bytes");
        assert_eq!(bytes[0], 0xEB, "{form}: {bytes:?}");
        let decoded_disp = decode_jmp_rel8(&bytes)
            .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
        assert_eq!(
            decoded_disp, displacement,
            "{form} round-tripped to disp {decoded_disp}, from bytes {bytes:?}"
        );
    }

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));
    assert!(
        vendor_source.contains("ICLASS    : JMP"),
        "pinned XED evidence must name ICLASS JMP"
    );
    assert!(
        vendor_source.contains("PATTERN   : 0xEB mode64"),
        "pinned XED evidence must contain the 0xEB rel8 JMP pattern"
    );
}

/// Independent decoder for Group-8 register/immediate bit operations:
/// REX.W(+B) + 0F BA + ModRM(mod=3,/4..7,rm) + imm8.
/// Reconstructed from bytes rather than sharing the encoder's helpers.
fn decode_group8_bitop_imm8(bytes: &[u8]) -> Option<(&'static str, u8, u8)> {
    let [rex, escape, opcode, modrm, bit_index] = bytes else {
        return None;
    };
    if (rex & 0xFE) != 0x48 || *escape != 0x0F || *opcode != 0xBA {
        return None;
    }
    if modrm >> 6 != 0b11 {
        return None;
    }
    let opcode_extension = (modrm >> 3) & 0b111;
    let mnemonic = match opcode_extension {
        4 => "bt",
        5 => "bts",
        6 => "btr",
        7 => "btc",
        _ => return None,
    };
    let register = (modrm & 0b111) | ((rex & 0x01) << 3);
    Some((mnemonic, register, *bit_index))
}

#[test]
fn lisp_group8_bitops_imm8_round_trip_independently_and_match_xed_extensions() {
    let mut session = encoder_session();
    let operations = [("bt", 4u8), ("bts", 5), ("btr", 6), ("btc", 7)];
    let registers = [("rax", 0u8), ("r9", 9), ("r15", 15)];

    for (mnemonic, expected_extension) in operations {
        for (register_name, register_code) in registers {
            for bit_index in [0u8, 3, 63, 255] {
                let form = format!(
                    "(x86-encode-{mnemonic}-r64-imm8 (quote {register_name}) {bit_index})"
                );
                let rendered = eval_bytes(&form, &mut session);
                let bytes: Vec<u8> = rendered
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .split_whitespace()
                    .map(|token| token.parse().expect("byte must be a small integer"))
                    .collect();

                assert_eq!(bytes.len(), 5, "{form} must be REX+0F+BA+ModRM+imm8");
                assert_eq!((bytes[3] >> 3) & 0b111, expected_extension, "{form}: {bytes:?}");

                let decoded = decode_group8_bitop_imm8(&bytes)
                    .unwrap_or_else(|| panic!("{form} produced undecodable bytes {bytes:?}"));
                assert_eq!(
                    decoded,
                    (mnemonic, register_code, bit_index),
                    "{form} round-tripped to {decoded:?} from bytes {bytes:?}"
                );
            }
        }
    }

    let vendor_path = repo_root().join("lib/machine/xed/vendor/base/xed-isa.txt");
    let vendor_source = fs::read_to_string(&vendor_path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", vendor_path.display()));

    for (iclass, extension_bits) in [
        ("BT", "100"),
        ("BTS", "101"),
        ("BTR", "110"),
        ("BTC", "111"),
    ] {
        assert!(
            vendor_source.contains(&format!("ICLASS    : {iclass}")),
            "pinned XED evidence must name {iclass}"
        );
        assert!(
            vendor_source.contains(&format!(
                "PATTERN   : 0x0F 0xBA MOD[0b11] MOD=3 REG[0b{extension_bits}] RM[nnn] UIMM8()"
            )),
            "pinned XED evidence must contain the register/immediate {iclass} Group-8 pattern"
        );
    }
}

#[test]
fn encoder_source_contains_no_process_or_assembler_escape_hatch() {
    let path = repo_root().join("lib/machine/encoding/x86-64.lisp");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
    let lower = source.to_ascii_lowercase();

    for forbidden in ["process-run", "nasm", " as ", "objcopy", "keystone", "iced-x86"] {
        assert!(
            !lower.contains(forbidden),
            "proof encoder must construct bytes in Lisp, not escape through {forbidden}"
        );
    }
}
