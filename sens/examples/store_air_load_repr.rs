//! Механічний witness для STORE/AIR: exact-width source -> production BitPacker.
//!
//! Цей helper не визначає семантику й не вибирає transport framing. Він лише
//! використовує чинні parse_binary_source_words/pack_binary_source_tokens та
//! виводить точний облік уже канонічного щільного payload.

use sens::{
    pack_binary_source_tokens, packed_transport_accounting, parse_binary_source_words,
    unpack_binary_source_words,
};
use std::{env, fs, process};

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "usage: {} <canonical-source-file> [framing-bits]",
            args.first()
                .map(String::as_str)
                .unwrap_or("store_air_load_repr")
        ));
    }

    let source = fs::read_to_string(&args[1])
        .map_err(|error| format!("read {}: {error}", args[1]))?;
    let framing_bits = if let Some(raw) = args.get(2) {
        raw.parse::<usize>()
            .map_err(|error| format!("invalid framing-bits: {error}"))?
    } else {
        0
    };

    let tokens =
        parse_binary_source_words(&source).map_err(|error| format!("parse: {error:?}"))?;
    if tokens.is_empty() {
        return Err("canonical source must contain at least one exact-width word".to_owned());
    }

    let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
    let expected_words: Vec<_> = tokens.iter().map(|token| token.word).collect();
    let packed = pack_binary_source_tokens(&tokens);
    let decoded = unpack_binary_source_words(&packed, &widths)
        .ok_or_else(|| "exact-width round-trip failed".to_owned())?;
    if decoded != expected_words {
        return Err("exact-width round-trip changed source words".to_owned());
    }
    let accounting = packed_transport_accounting(&packed, framing_bits);
    let utilization = accounting
        .utilization()
        .ok_or_else(|| "non-empty payload unexpectedly has no utilization".to_owned())?;

    println!("SEMANTIC_WORD_COUNT={}", tokens.len());
    println!("SEMANTIC_PAYLOAD_BITS={}", accounting.semantic_payload_bits);
    println!("FRAMING_BITS={}", accounting.framing_bits);
    println!("TAIL_UNUSED_BITS={}", accounting.tail_unused_bits);
    println!("BYTE_CONTAINER_TOTAL_BITS={}", accounting.total_wire_bits);
    println!("PHYSICAL_CONTAINER_BYTES={}", packed.byte_len());
    println!("PAYLOAD_UTILIZATION={utilization:.17}");
    println!("ROUNDTRIP_WORDS_OK=1");

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR: {error}");
        process::exit(2);
    }
}
