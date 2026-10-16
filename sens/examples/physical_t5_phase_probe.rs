//! In-process physical SENS measurement, not a replacement for end-to-end CLI.
//!
//! Reports independent positive phase costs; does NOT subtract noisy medians
//! or claim the hot evaluator time includes startup, I/O, or Core4 bootstrap.
//! The same physical bytes must decode, re-encode, parse and execute first.
use sens::{
    decode_ternary_words, encode_ternary_words, eval_parsed_expressions,
    parse_canonical_word_sequence, Session,
};
use std::{env, fs, hint::black_box, process::ExitCode, time::Instant};

const PHASES: [&str; 6] = [
    "session", "decode_t5", "parse_d2", "eval_hot",
    "eval_fresh", "full_in_memory",
];

fn hex_value(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        use std::fmt::Write;
        write!(&mut out, "{byte:02x}").expect("hex formatting");
    }
    out
}

fn main() -> ExitCode {
    if let Err(error) = measure() {
        eprintln!("BLOCK: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn measure() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let source = args.next().ok_or("usage: physical_t5_phase_probe FILE.sens WARMUP REPS INNER")?;
    let warmup: usize = args.next().ok_or("missing warmup")?
        .parse().map_err(|_| "warmup must be integer")?;
    let reps: usize = args.next().ok_or("missing reps")?
        .parse().map_err(|_| "reps must be integer")?;
    let inner: usize = args.next().ok_or("missing inner")?
        .parse().map_err(|_| "inner must be integer")?;
    if args.next().is_some() || reps < 5 || inner == 0 || inner > 100_000 || warmup > 100 {
        return Err("expected FILE.sens WARMUP REPS INNER, reps>=5 and 1<=inner<=100000".into());
    }
    if !source.ends_with(".sens") {
        return Err("physical input must have .sens extension".into());
    }

    // Disk I/O is explicitly OUTSIDE the timed phases.
    let bytes = fs::read(&source).map_err(|error| format!("read: {error}"))?;
    let words = decode_ternary_words(&bytes).map_err(|error| format!("T5 decode: {error:?}"))?;
    if encode_ternary_words(&words).map_err(|error| format!("T5 encode: {error:?}"))? != bytes {
        return Err("physical byte parity mismatch".into());
    }
    let forms = parse_canonical_word_sequence(&words)
        .map_err(|error| format!("D2 syntax: {error}"))?;
    let mut truth_session = Session::default();
    let expected = eval_parsed_expressions(&forms, &mut truth_session)
        .map_err(|error| format!("SENS oracle: {error}"))?
        .value.to_string();
    if expected.is_empty() {
        return Err("empty observable result is not an admitted benchmark oracle".into());
    }

    println!("PROBE\tversion\t1");
    println!("ORACLE_HEX\t{}", hex_value(&expected));
    println!("WORDS\t{}", words.len());
    println!("T5_BYTES\t{}", bytes.len());
    // Only eval_hot reuses its already-ready, capability-free session.
    let mut hot_session = Session::default();

    for sample in 0..(warmup + reps) {
        // Rotate so one phase never always receives the same cache position.
        for turn in 0..PHASES.len() {
            let phase = PHASES[(turn + sample) % PHASES.len()];
            let started = Instant::now();
            for _ in 0..inner {
                match phase {
                    "session" => {
                        black_box(Session::default());
                    }
                    "decode_t5" => {
                        let decoded = decode_ternary_words(black_box(&bytes))
                            .map_err(|e| format!("T5 in loop: {e:?}"))?;
                        black_box(decoded);
                    }
                    "parse_d2" => {
                        let parsed = parse_canonical_word_sequence(black_box(&words))
                            .map_err(|e| format!("D2 in loop: {e}"))?;
                        black_box(parsed);
                    }
                    "eval_hot" => {
                        let result = eval_parsed_expressions(black_box(&forms), &mut hot_session)
                            .map_err(|e| format!("hot execution: {e}"))?;
                        black_box(result);
                    }
                    "eval_fresh" => {
                        let mut session = Session::default();
                        let result = eval_parsed_expressions(black_box(&forms), &mut session)
                            .map_err(|e| format!("fresh execution: {e}"))?;
                        black_box(result);
                    }
                    "full_in_memory" => {
                        let decoded = decode_ternary_words(black_box(&bytes))
                            .map_err(|e| format!("full T5: {e:?}"))?;
                        let parsed = parse_canonical_word_sequence(&decoded)
                            .map_err(|e| format!("full D2: {e}"))?;
                        let mut session = Session::default();
                        let result = eval_parsed_expressions(&parsed, &mut session)
                            .map_err(|e| format!("full eval: {e}"))?;
                        black_box(result);
                    }
                    _ => unreachable!(),
                }
            }
            let elapsed = started.elapsed().as_nanos();
            if sample >= warmup {
                // Timing does not include string serialization or this oracle.
                let mut verify_session = Session::default();
                let got = eval_parsed_expressions(&forms, &mut verify_session)
                    .map_err(|e| format!("per-sample oracle: {e}"))?
                    .value.to_string();
                if got != expected {
                    return Err(format!("nondeterministic execution in {phase} at {sample}"));
                }
                println!("SAMPLE\t{phase}\t{}\t{inner}\t{elapsed}", sample - warmup);
            }
        }
    }
    Ok(())
}
