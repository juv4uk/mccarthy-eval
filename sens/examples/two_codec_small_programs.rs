//! Two *explicit* research physical codecs: canonical T5 for ordinary programs,
//! Frame3 Carrier (F3C, historical `.senc`) for very small programs.
//! NOT Envelope E1/E2/E3 and NOT the one-byte Frame3-Micro (F3M, `.f3m`).
//!
//! Does not modify the production .sens loader, the D1-D9 domains, or the
//! existing SENS execution path. Convert only already-admitted physical T5.
//! Select F3 only when the WHOLE file (including 0xF3 label) is shorter,
//! the input T5 is at most 16 bytes, and a real F3 exact-word roundtrip passes.
//! Unknown/ambiguous extensions never trigger content-based sniffing.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
};

use sens::{
    decode_ternary_words, encode_ternary_words, eval_parsed_expressions,
    parse_binary_source_words, parse_canonical_binary, parse_canonical_word_sequence,
    PhysicalT5Program, Session,
};

#[path = "../../../research/framed3/rust_codec/senc.rs"]
mod senc;

const MAX_SMALL_T5_BYTES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Codec {
    T5,
    OriginalFrame3,
}

impl Codec {
    fn extension(self) -> &'static str {
        match self {
            Self::T5 => "sens",
            Self::OriginalFrame3 => "senc",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::T5 => "T5",
            Self::OriginalFrame3 => "F3",
        }
    }
}

struct Choice {
    codec: Codec,
    physical: Vec<u8>,
    reason: &'static str,
    available_f3_bytes: Option<usize>,
}

fn read_t5_canon(raw: &[u8]) -> Result<Vec<sens::BinarySourceWord>, String> {
    let p = PhysicalT5Program::decode(raw)
        .map_err(|e| format!("production T5 reader rejection: {e:?}"))?;
    if p.form_count() == 0 {
        return Err("zero canonical forms: not a program".into());
    }
    let words = decode_ternary_words(raw)
        .map_err(|e| format!("T5 transport: {e:?}"))?;
    let repeated = encode_ternary_words(&words)
        .map_err(|e| format!("T5 re-encode: {e:?}"))?;
    if repeated != raw {
        return Err("noncanonical T5 physical bytes".into());
    }
    parse_canonical_word_sequence(&words)
        .map_err(|e| format!("D2 canonical grammar: {e:?}"))?;
    Ok(words)
}

fn exact_strings(words: &[sens::BinarySourceWord]) -> Vec<String> {
    words.iter().map(|w| {
        format!("{:0width$b}", w.packed_bits(), width = w.width())
    }).collect()
}

fn select_from_t5(raw: &[u8]) -> Result<Choice, String> {
    let words = read_t5_canon(raw)?;
    let mut choice = Choice {
        codec: Codec::T5,
        physical: raw.to_vec(),
        reason: "T5_DEFAULT",
        available_f3_bytes: None,
    };
    if raw.len() > MAX_SMALL_T5_BYTES {
        choice.reason = "T5_OVER_16B";
        return Ok(choice);
    }
    let exact = exact_strings(&words);
    let Ok(body) = senc::frame_encode(&exact) else {
        choice.reason = "T5_F3_UNSUPPORTED";
        return Ok(choice);
    };
    let mut framed = vec![senc::FRAME_LABEL];
    framed.extend_from_slice(&body);
    if !matches!(senc::decode_senc(&framed), Ok(restored) if restored == exact) {
        // Original research F3 has known noncanonical / parity gaps; never
        // let an optimistic frame_encode overwrite a valid T5 program.
        choice.reason = "T5_F3_NOT_CANONICAL";
        return Ok(choice);
    }
    choice.available_f3_bytes = Some(framed.len());
    if framed.len() >= raw.len() {
        choice.reason = "T5_SMALLER_OR_EQUAL";
        return Ok(choice);
    }
    // Independent real typed-word bridge and canonical physical T5 oracle.
    let source = exact.join(" ");
    let retyped = parse_binary_source_words(&source)
        .map_err(|e| format!("source word parse: {e:?}"))?
        .into_iter().map(|w| w.word).collect::<Vec<_>>();
    if encode_ternary_words(&retyped)
        .map_err(|e| format!("independent bridge encode: {e:?}"))? != raw
    {
        return Err("F3 bridge not bit-exact against production physical T5".into());
    }
    Ok(Choice {
        codec: Codec::OriginalFrame3,
        physical: framed,
        reason: "F3_SMALLER_VERIFIED",
        available_f3_bytes: Some(body.len() + 1),
    })
}

/// The extension is an explicit selection of the decoder, never an autodetect
/// or a fallback if one codec fails. Return canonical production T5 bytes.
fn explicit_decode(raw: &[u8], extension: &str) -> Result<Vec<u8>, String> {
    match extension {
        "sens" => {
            read_t5_canon(raw)?;
            Ok(raw.to_vec())
        }
        "senc" => {
            if raw.first() != Some(&senc::FRAME_LABEL) {
                return Err("F3 extension requires original 0xF3 marker".into());
            }
            let exact = senc::decode_senc(raw)
                .map_err(|e| format!("strict original F3 decoder: {e:?}"))?;
            let source = exact.join(" ");
            let words = parse_binary_source_words(&source)
                .map_err(|e| format!("F3 exact words: {e:?}"))?
                .into_iter().map(|t| t.word).collect::<Vec<_>>();
            let reconstructed = encode_ternary_words(&words)
                .map_err(|e| format!("F3 -> production T5: {e:?}"))?;
            read_t5_canon(&reconstructed)?;
            let selected = select_from_t5(&reconstructed)?;
            if selected.codec != Codec::OriginalFrame3 || selected.physical != raw {
                return Err("F3 violates canonical small-file selection".into());
            }
            Ok(reconstructed)
        }
        _ => Err("unknown codec/extension; only explicit .sens or .senc accepted".into()),
    }
}

fn verify_roundtrip(raw: &[u8], selection: &Choice) -> Result<(), String> {
    let reproduced = explicit_decode(&selection.physical, selection.codec.extension())?;
    if reproduced != raw {
        return Err("different physical T5 bytes after two-codec exact roundtrip".into());
    }
    if selection.codec == Codec::OriginalFrame3 {
        if raw.len() > MAX_SMALL_T5_BYTES || selection.physical.len() >= raw.len() {
            return Err("F3 was selected despite size/eligibility guard".into());
        }
    }
    Ok(())
}

fn self_test() -> Result<(), String> {
    // This closed, proper D3 call is not a residual ((0)) expression.
    let sample = "10 001 00 000 01"; // QUOTE(000), D2 separator retained.
    let words = parse_binary_source_words(sample)
        .map_err(|e| format!("self-test source: {e:?}"))?
        .into_iter().map(|x| x.word).collect::<Vec<_>>();
    let raw = encode_ternary_words(&words)
        .map_err(|e| format!("self-test T5: {e:?}"))?;
    let chosen = select_from_t5(&raw)?;
    verify_roundtrip(&raw, &chosen)?;
    // Prove this is executable SENS code, not merely a balanced D2 tree.
    // Execute the same admitted QUOTE program independently after both routes.
    let parsed = parse_canonical_binary(sample)
        .map_err(|e| format!("SENS AST: {e:?}"))?;
    let expected = eval_parsed_expressions(&parsed, &mut Session::default())
        .map_err(|e| format!("direct SENS oracle: {e:?}"))?;
    let recovered = explicit_decode(&chosen.physical, "senc")?;
    let physical_program = PhysicalT5Program::decode(&recovered)
        .map_err(|e| format!("recovered physical SENS program: {e:?}"))?;
    let actual = physical_program.execute(&mut Session::default())
        .map_err(|e| format!("recovered SENS execution: {e:?}"))?;
    if expected.value != actual.value || expected.output != actual.output {
        return Err("F3 decoded program evaluation differs from canonical SENS oracle".into());
    }
    if chosen.codec != Codec::OriginalFrame3 {
        return Err(format!("small F3 proof was not selected: {}", chosen.reason));
    }
    if explicit_decode(&chosen.physical, "sens").is_ok() {
        return Err("F3 body was silently accepted as T5".into());
    }
    if explicit_decode(&raw, "senc").is_ok() {
        return Err("T5 body was silently accepted as F3".into());
    }
    if explicit_decode(&chosen.physical, "unknown").is_ok()
        || explicit_decode(&[0xf4, 0], "senc").is_ok()
        || explicit_decode(&[], "senc").is_ok()
        || explicit_decode(&[0xff], "sens").is_ok()
        || explicit_decode(&chosen.physical[..1], "senc").is_ok()
    {
        return Err("invalid/corrupted carrier was accepted".into());
    }
    // Over-threshold fully canonical source must stay on T5 even if F3
    // could offer a shorter body. No arbitrary forced F3 execution.
    let mut large_text = Vec::new();
    for _ in 0..50 { large_text.push("1"); }
    let huge_words = parse_binary_source_words(&large_text.join(" "))
        .map_err(|e| format!("large source: {e:?}"))?
        .into_iter().map(|t| t.word).collect::<Vec<_>>();
    let huge = encode_ternary_words(&huge_words)
        .map_err(|e| format!("large T5: {e:?}"))?;
    let large = select_from_t5(&huge)?;
    if large.codec != Codec::T5 || large.reason != "T5_OVER_16B" {
        return Err("large T5 was not preserved".into());
    }
    verify_roundtrip(&huge, &large)?;
    eprintln!("TWO_CODEC_SELFTEST_PASS: executable small F3, large T5, exact physical T5 roundtrip and strict negative carriers");
    Ok(())
}

fn report() -> Result<(), String> {
    self_test()?;
    let git = Command::new("git").args(["ls-files", "-z", "--", "*.sens"])
        .output().map_err(|e| format!("git ls-files: {e}"))?;
    if !git.status.success() {
        return Err("failed to collect full git-tracked physical .sens inventory".into());
    }
    let mut paths = git.stdout.split(|&b| b == 0).filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    if paths.is_empty() {
        return Err("no tracked physical files; no supporting proof".into());
    }
    println!("# policy=canonical T5 .sens by default; F3 .senc only <=16B original T5 and strict smaller full carrier");
    println!("# profile=original research F3 with 0xF3 byte label; not S3/v1-v3; not .f3m micro profile");
    println!("# extension-dispatched strict decoder; canonical production T5 proof; no implicit fallback");
    println!("file\tt5_bytes\tcodec\tselected_bytes\textension\treason\tverified_f3_bytes");
    let (mut scanned, mut small, mut f3, mut original, mut selected) = (0, 0, 0, 0, 0);
    for path in paths {
        let raw = fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
        let c = select_from_t5(&raw).map_err(|e| format!("{path}: {e}"))?;
        verify_roundtrip(&raw, &c).map_err(|e| format!("{path}: {e}"))?;
        scanned += 1;
        if raw.len() <= MAX_SMALL_T5_BYTES { small += 1; }
        if c.codec == Codec::OriginalFrame3 { f3 += 1; }
        original += raw.len();
        selected += c.physical.len();
        let candidate = c.available_f3_bytes
            .map(|n| n.to_string()).unwrap_or_else(|| "NA".into());
        println!("{path}\t{}\t{}\t{}\t.{}\t{}\t{}",
                 raw.len(), c.codec.name(), c.physical.len(), c.codec.extension(),
                 c.reason, candidate);
    }
    eprintln!("TWO_CODEC_REAL_FILE_PASS: tracked={scanned}, small_T5_<=16B={small}, selected_F3={f3}, original_T5={original}B, selected_total={selected}B, all_exact_roundtrips={scanned}");
    if small == 0 || f3 == 0 || selected >= original {
        return Err("two-codec policy did not save physical bytes on current corpus".into());
    }
    Ok(())
}

fn convert(input: &Path, output_stem: &Path) -> Result<(), String> {
    if input.extension().and_then(|e| e.to_str()) != Some("sens") {
        return Err("input must explicitly be an existing canonical .sens".into());
    }
    if output_stem.extension().is_some() {
        return Err("output must be a stem without extension, to keep codec explicit".into());
    }
    let raw = fs::read(input).map_err(|e| format!("read input: {e}"))?;
    let c = select_from_t5(&raw)?;
    verify_roundtrip(&raw, &c)?;
    let destination = output_stem.with_extension(c.codec.extension());
    if destination == input {
        return Err("refuse to overwrite source file".into());
    }
    let mut f = OpenOptions::new().write(true).create_new(true).open(&destination)
        .map_err(|e| format!("create-new {}: {e}", destination.display()))?;
    f.write_all(&c.physical).map_err(|e| format!("write: {e}"))?;
    f.sync_all().map_err(|e| format!("sync: {e}"))?;
    let on_disk = fs::read(&destination).map_err(|e| format!("verify read: {e}"))?;
    if on_disk != c.physical {
        return Err("written file differs from verified bytes".into());
    }
    println!("{}\t{}\t{}B\t{}\t{}",
        destination.display(), c.codec.name(), c.physical.len(),
        c.reason, c.codec.extension());
    Ok(())
}

/// Offline-only exact restore of an explicitly selected file. This must never
/// be wired into production .sens content auto-detection.
fn restore_to_t5(input: &Path, output_stem: &Path) -> Result<(), String> {
    let extension = input.extension().and_then(|x| x.to_str())
        .ok_or("missing .sens/.senc source extension")?;
    if output_stem.extension().is_some() {
        return Err("restore destination must be an extension-free output stem".into());
    }
    let raw = fs::read(input).map_err(|e| format!("restore source: {e}"))?;
    let restored = explicit_decode(&raw, extension)?;
    read_t5_canon(&restored)?;
    let destination = output_stem.with_extension("sens");
    if destination == input {
        return Err("restore cannot overwrite its input".into());
    }
    let mut out = OpenOptions::new().write(true).create_new(true)
        .open(&destination).map_err(|e| format!("restore create-new: {e}"))?;
    out.write_all(&restored).map_err(|e| format!("restore write: {e}"))?;
    out.sync_all().map_err(|e| format!("restore sync: {e}"))?;
    if fs::read(&destination).map_err(|e| format!("restore read: {e}"))? != restored {
        return Err("restored physical bytes disagree with decoded T5".into());
    }
    println!("{}\tRESTORED_T5\t{}B", destination.display(), restored.len());
    Ok(())
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [flag] if flag == "--self-test" => self_test(),
        [flag] if flag == "--report" => report(),
        [flag, source, target] if flag == "--convert" =>
            convert(Path::new(source), Path::new(target)),
        [flag, source, target] if flag == "--restore" =>
            restore_to_t5(Path::new(source), Path::new(target)),
        _ => Err("usage: two_codec_small_programs --self-test | --report | --convert in.sens OUT_STEM | --restore in.(sens|senc) OUT_STEM".into()),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("TWO_CODEC_BLOCKED: {e}");
        std::process::exit(1);
    }
}
