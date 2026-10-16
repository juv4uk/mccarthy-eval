const REGISTERS: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11",
    "r12", "r13", "r14", "r15",
];

fn register_name(code: u8) -> Result<&'static str, String> {
    REGISTERS
        .get(code as usize)
        .copied()
        .ok_or_else(|| format!("invalid GPR code {code}"))
}

fn rex_w_no_index(byte: u8) -> bool {
    (byte & 0xF0) == 0x40 && (byte & 0x08) != 0 && (byte & 0x02) == 0
}

fn decode_mov_r64_imm64(bytes: &[u8], offset: usize) -> Result<Option<(String, usize)>, String> {
    let Some(&rex) = bytes.get(offset) else {
        return Ok(None);
    };
    if rex != 0x48 && rex != 0x49 {
        return Ok(None);
    }

    let Some(&opcode) = bytes.get(offset + 1) else {
        return Err(format!("truncated instruction after REX at byte {offset}"));
    };
    if !(0xB8..=0xBF).contains(&opcode) {
        return Ok(None);
    }

    let end = offset + 10;
    let immediate_bytes = bytes
        .get(offset + 2..end)
        .ok_or_else(|| format!("truncated MOV r64, imm64 at byte {offset}"))?;
    let mut little_endian = [0u8; 8];
    little_endian.copy_from_slice(immediate_bytes);
    let immediate = u64::from_le_bytes(little_endian);

    let register_code = (opcode - 0xB8) | ((rex & 0x01) << 3);
    let register = register_name(register_code)?;
    Ok(Some((
        format!("(mov-r64-imm64 {register} {immediate})"),
        end,
    )))
}

fn decode_mov_mem_disp8(bytes: &[u8], offset: usize) -> Result<Option<(String, usize)>, String> {
    let Some(&rex) = bytes.get(offset) else {
        return Ok(None);
    };
    if !rex_w_no_index(rex) {
        return Ok(None);
    }

    let Some(&opcode) = bytes.get(offset + 1) else {
        return Err(format!("truncated instruction after REX at byte {offset}"));
    };
    if opcode != 0x8B && opcode != 0x89 {
        return Ok(None);
    }

    let Some(&modrm) = bytes.get(offset + 2) else {
        return Err(format!("truncated MOV memory ModR/M at byte {offset}"));
    };
    if (modrm >> 6) != 0b01 {
        return Err(format!(
            "MOV memory observer only admits mod=01 disp8 at byte {offset}"
        ));
    }

    let data_code = ((modrm >> 3) & 0b111) | (((rex >> 2) & 0x01) << 3);
    let data_register = register_name(data_code)?;
    let rm = modrm & 0b111;
    let rex_b = (rex & 0x01) << 3;

    let (base_code, displacement_offset, next) = if rm == 0b100 {
        let Some(&sib) = bytes.get(offset + 3) else {
            return Err(format!("truncated MOV memory SIB at byte {offset}"));
        };
        let scale = sib >> 6;
        let index = (sib >> 3) & 0b111;
        let base = sib & 0b111;
        if scale != 0 || index != 0b100 || base != 0b100 {
            return Err(format!(
                "MOV memory observer requires no-index rsp/r12 SIB at byte {offset}"
            ));
        }
        (base | rex_b, offset + 4, offset + 5)
    } else {
        (rm | rex_b, offset + 3, offset + 4)
    };

    let base_register = register_name(base_code)?;
    let Some(&disp_byte) = bytes.get(displacement_offset) else {
        return Err(format!("truncated MOV memory disp8 at byte {offset}"));
    };
    let displacement = i8::from_ne_bytes([disp_byte]) as i32;

    let form = if opcode == 0x8B {
        format!(
            "(mov-r64-mem-disp8 {data_register} {base_register} {displacement})"
        )
    } else {
        format!(
            "(mov-mem-disp8-r64 {base_register} {displacement} {data_register})"
        )
    };

    Ok(Some((form, next)))
}

fn alu_name(opcode: u8) -> Option<&'static str> {
    match opcode {
        0x01 => Some("add"),
        0x09 => Some("or"),
        0x21 => Some("and"),
        0x29 => Some("sub"),
        0x31 => Some("xor"),
        0x39 => Some("cmp"),
        _ => None,
    }
}

fn decode_alu_r64_r64(bytes: &[u8], offset: usize) -> Result<Option<(String, usize)>, String> {
    let Some(&rex) = bytes.get(offset) else {
        return Ok(None);
    };
    if !rex_w_no_index(rex) {
        return Ok(None);
    }

    let Some(&opcode) = bytes.get(offset + 1) else {
        return Err(format!("truncated instruction after REX at byte {offset}"));
    };
    let Some(name) = alu_name(opcode) else {
        return Ok(None);
    };
    let Some(&modrm) = bytes.get(offset + 2) else {
        return Err(format!("truncated {name} r64, r64 at byte {offset}"));
    };
    if (modrm >> 6) != 0b11 {
        return Err(format!(
            "{name} observer only admits the register/register form at byte {offset}"
        ));
    }

    let destination_code = (modrm & 0b111) | ((rex & 0x01) << 3);
    let source_code = ((modrm >> 3) & 0b111) | (((rex >> 2) & 0x01) << 3);
    let destination = register_name(destination_code)?;
    let source = register_name(source_code)?;

    Ok(Some((
        format!("({name}-r64-r64 {destination} {source})"),
        offset + 3,
    )))
}

// #196 observer for the one conditional-transfer form demanded by the bounded
// EQ+COND witness. This intentionally decodes the physical 0x75 shape from
// scratch rather than sharing the Lisp encoder's Jcc table.
fn decode_jnz_rel8(bytes: &[u8], offset: usize) -> Result<Option<(String, usize)>, String> {
    if bytes.get(offset).copied() != Some(0x75) {
        return Ok(None);
    }
    let displacement = *bytes
        .get(offset + 1)
        .ok_or_else(|| format!("truncated JNZ rel8 at byte {offset}"))?;
    let signed = i8::from_ne_bytes([displacement]);
    Ok(Some((format!("(jnz-rel8 {signed})"), offset + 2)))
}

pub fn decode_machine_block(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut forms = Vec::new();
    let mut offset = 0usize;

    while offset < bytes.len() {
        if bytes[offset] == 0xC3 {
            forms.push("(ret)".to_string());
            offset += 1;
            continue;
        }

        if let Some((form, next)) = decode_mov_r64_imm64(bytes, offset)? {
            forms.push(form);
            offset = next;
            continue;
        }

        if let Some((form, next)) = decode_mov_mem_disp8(bytes, offset)? {
            forms.push(form);
            offset = next;
            continue;
        }

        if let Some((form, next)) = decode_alu_r64_r64(bytes, offset)? {
            forms.push(form);
            offset = next;
            continue;
        }

        if let Some((form, next)) = decode_jnz_rel8(bytes, offset)? {
            forms.push(form);
            offset = next;
            continue;
        }

        return Err(format!(
            "unsupported or malformed x86-64 byte sequence at offset {offset}: 0x{:02x}",
            bytes[offset]
        ));
    }

    Ok(forms)
}
