//! The SENS host -- ONE small file.
//!
//! It attaches to the sterile Zig corridor (Zig ABI v2) and does nothing else:
//! no bit-packing (Zig owns it), no domain-ladder reader (the SENS REPL owns it),
//! no arithmetic (SENS owns it). Rust is byte-addressed; it only hands bytes to
//! Zig and receives bytes back. The domain ladder crosses the corridor verbatim.

use std::process::ExitCode;

// ---- the sterile Zig corridor (ABI v2) --------------------------------------
unsafe extern "C" {
    fn sens_zig_codec_abi_version() -> u32;
    fn sens_zig_t5_validate(src: *const u8, len: usize) -> i32;
    fn sens_zig_t5_encode_be3(src: *const u8, len: usize, dst: *mut u8, cap: usize) -> isize;
    fn sens_zig_t5_decode_be3(src: *const u8, len: usize, dst: *mut u8, cap: usize) -> isize;
}

const ABI: u32 = 2;

fn main() -> ExitCode {
    // The host knows the ABI version and the domain ladder -- nothing else.
    if unsafe { sens_zig_codec_abi_version() } != ABI {
        eprintln!("sens-host: Zig ABI mismatch (want {ABI})");
        return ExitCode::FAILURE;
    }

    // Read bytes (already-packed programs come from Zig) and hand them over.
    let mut input = Vec::new();
    if std::io::Read::read_to_end(&mut std::io::stdin(), &mut input).is_err() {
        return ExitCode::FAILURE;
    }

    if unsafe { sens_zig_t5_validate(input.as_ptr(), input.len()) } != 0 {
        eprintln!("sens-host: not a valid T5 carrier");
        return ExitCode::FAILURE;
    }

    // Round-trip through Zig (the only intelligence is the codec).
    let mut be3 = vec![0u8; input.len() * 4];
    let n = unsafe { sens_zig_t5_encode_be3(input.as_ptr(), input.len(), be3.as_mut_ptr(), be3.len()) };
    if n < 0 {
        return ExitCode::FAILURE;
    }
    let mut back = vec![0u8; input.len()];
    let m = unsafe { sens_zig_t5_decode_be3(be3.as_ptr(), n as usize, back.as_mut_ptr(), back.len()) };
    if m < 0 {
        return ExitCode::FAILURE;
    }

    std::io::Write::write_all(&mut std::io::stdout(), &back[..m as usize]).ok();
    ExitCode::SUCCESS
}
