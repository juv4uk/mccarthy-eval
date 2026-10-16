//! Dense F3M-1 proof: every physical byte names one closed SENS program.
//!
//! The active Frame3-Micro enumerator is shared with F3M-2/F3M-3 and uses
//! disjoint global-rank windows selected only by physical length.  This witness
//! exhausts all 256 one-byte payloads, verifies unique decode/encode, canonical
//! SENS parsing, real evaluator execution and independent physical T5 parity.
//! There are no reserved/magic byte values in active F3M-1.

use std::{collections::HashSet, path::Path};

use sens::{
    encode_ternary_words, eval_parsed_expressions,
    frame3_micro_enumerator::{decode_payload, encode_expr, Frame3MicroExpr},
    parse_binary_source_words, parse_canonical_binary, PhysicalT5Program, Session,
};

fn verify_one(code: u8) -> Result<(usize, String, Frame3MicroExpr), String> {
    let payload = [code];
    let expr = decode_payload(&payload).map_err(|error| format!("{error:?}"))?;
    if encode_expr(&expr).map_err(|error| format!("{error:?}"))? != payload {
        return Err(format!("noncanonical dense F3M-1 rank: {code:02x}"));
    }

    let source = expr.exact_words().join(" ");
    let parsed = parse_canonical_binary(&source).map_err(|error| format!("D2: {error:?}"))?;
    if parsed.len() != 1 {
        return Err(format!("not exactly one canonical form: {source}"));
    }

    let typed = parse_binary_source_words(&source)
        .map_err(|error| format!("exact typed words: {error:?}"))?
        .into_iter()
        .map(|token| token.word)
        .collect::<Vec<_>>();
    let t5 = encode_ternary_words(&typed).map_err(|error| format!("T5: {error:?}"))?;

    let expected = eval_parsed_expressions(&parsed, &mut Session::default())
        .map_err(|error| format!("evaluator: {error:?}"))?;
    let t5_program = PhysicalT5Program::decode(&t5)
        .map_err(|error| format!("physical reader: {error:?}"))?;
    if t5_program.form_count() != 1 {
        return Err(format!("not one physical T5 form: {source}"));
    }
    let actual = t5_program
        .execute(&mut Session::default())
        .map_err(|error| format!("physical T5 execution: {error:?}"))?;
    if expected.value != actual.value || expected.output != actual.output {
        return Err(format!("SENS evaluator ↔ physical T5 mismatch: {source}"));
    }

    Ok((t5.len(), format!("{:?}", actual.value), expr))
}

fn bench_speed_one(
    label: &str,
    mut operation: impl FnMut(),
    samples: usize,
    inner: usize,
) {
    use std::{hint::black_box, time::Instant};

    for _ in 0..inner {
        operation();
    }
    let mut observations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..inner {
            operation();
        }
        let elapsed = start.elapsed().as_nanos();
        black_box(elapsed);
        observations.push(elapsed as f64 / inner as f64);
    }
    observations.sort_by(f64::total_cmp);
    let p50 = observations[samples / 2];
    let p95 = observations[(samples * 95).div_ceil(100).saturating_sub(1)];
    println!("{label}\t{p50:.2}\t{p95:.2}");
}

fn bench_speed() -> Result<(), Box<dyn std::error::Error>> {
    use std::hint::black_box;

    const SAMPLES: usize = 19;
    const INNER: usize = 300;
    // Under the dense active order CAR(CONS(1,NIL)) moved from the historical
    // sparse witness 0x45 to canonical global rank 62 = 0x3e.
    let one_byte = [0x3e];
    let expr = decode_payload(&one_byte).map_err(|error| format!("{error:?}"))?;
    if expr.human_projection() != "CAR(CONS(1,NIL))" {
        return Err("dense F3M-1 trophy rank drifted".into());
    }
    let exact = expr.exact_words();
    let visible = exact.join(" ");
    let tokens = parse_binary_source_words(&visible)?;
    let typed = tokens.iter().map(|token| token.word).collect::<Vec<_>>();
    let full_t5 = encode_ternary_words(&typed)
        .map_err(|error| format!("T5 physical encode: {error:?}"))?;
    if full_t5.len() != 5 || encode_expr(&expr)? != one_byte {
        return Err("wrong physical size or broken dense roundtrip".into());
    }
    let expected = eval_parsed_expressions(
        &parse_canonical_binary(&visible)?,
        &mut Session::default(),
    )?;
    let t5_value = PhysicalT5Program::decode(&full_t5)
        .map_err(|error| format!("T5 physical decode: {error:?}"))?
        .execute(&mut Session::default())?;
    if expected != t5_value {
        return Err("real evaluator mismatch".into());
    }

    println!("# exact_program=CAR(CONS(1,NIL)), same D1/D2/D3 typed word sequence");
    println!("# active_profile=F3M-1 selected by exact physical length");
    println!("# physical_bytes_f3m=1 physical_bytes_t5=5");
    println!("# machine=Github hosted runner, rust --release, buffer-only, no file I/O");
    println!("# samples={SAMPLES}, inner={INNER}, warmup={INNER}, units=ns/op; p95 nearest rank");
    println!("operation\tp50_ns\tp95_ns");
    bench_speed_one(
        "f3m_dense_encode_exact_program",
        || {
            black_box(encode_expr(black_box(&expr)).unwrap());
        },
        SAMPLES,
        INNER,
    );
    bench_speed_one(
        "t5_encode_typed_words",
        || {
            black_box(encode_ternary_words(black_box(&typed)).unwrap());
        },
        SAMPLES,
        INNER,
    );
    bench_speed_one(
        "f3m_dense_decode_1b_to_program",
        || {
            black_box(decode_payload(black_box(&one_byte)).unwrap());
        },
        SAMPLES,
        INNER,
    );
    bench_speed_one(
        "t5_decode_5b_to_typed_words",
        || {
            black_box(sens::decode_ternary_words(black_box(&full_t5)).unwrap());
        },
        SAMPLES,
        INNER,
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const TRACKED_ONE_BYTE_FILE: &[u8] =
        include_bytes!("../../../research/framed3/fixtures/car-cons-one-nil.f3m");
    if TRACKED_ONE_BYTE_FILE != [0x3e] {
        return Err("Git-tracked active F3M-1 trophy is not exactly byte 0x3e".into());
    }

    if std::env::args().nth(1).as_deref() == Some("--speed") {
        return bench_speed();
    }

    let mut seen = HashSet::new();
    println!("code_hex\tmicro_file_bytes\tt5_full_bytes\tdecoded_exact_words\tactual_sens_value");
    for code in 0u8..=u8::MAX {
        let (t5_len, result, expr) = verify_one(code)?;
        if !seen.insert(expr.clone()) {
            return Err(format!("duplicate dense F3M-1 program at {code:02x}").into());
        }
        println!(
            "{code:02x}\t1\t{t5_len}\t{}\t{result}",
            expr.exact_words().join(" ")
        );
    }
    if seen.len() != 256 {
        return Err(format!("dense F3M-1 did not cover all 256 codes: {}", seen.len()).into());
    }

    let trophy = decode_payload(&[0x3e]).map_err(|error| format!("{error:?}"))?;
    if trophy.human_projection() != "CAR(CONS(1,NIL))" {
        return Err("dense trophy did not reconstruct CAR(CONS(1,NIL))".into());
    }
    let expected = eval_parsed_expressions(
        &parse_canonical_binary("1")?,
        &mut Session::default(),
    )?;
    let actual = eval_parsed_expressions(
        &parse_canonical_binary(&trophy.exact_words().join(" "))?,
        &mut Session::default(),
    )?;
    if expected.value != actual.value || !actual.output.is_empty() {
        return Err("CAR(CONS(1,NIL)) did not compute 1 in real evaluator".into());
    }

    if let Some(target) = std::env::args().nth(1) {
        let target = Path::new(&target);
        std::fs::create_dir_all(target)?;
        std::fs::write(target.join("car-cons-one-nil.f3m"), [0x3e])?;
        let atom_nil = encode_expr(&Frame3MicroExpr::Atom(Box::new(Frame3MicroExpr::Nil)))?;
        if atom_nil.len() != 1 {
            return Err("ATOM(NIL) did not remain in F3M-1".into());
        }
        std::fs::write(target.join("atom-nil.f3m"), atom_nil)?;
        if std::fs::metadata(target.join("car-cons-one-nil.f3m"))?.len() != 1 {
            return Err("actual physical carrier is not 1 byte".into());
        }
    }

    eprintln!(
        "F3M1_DENSE_PASS: 256/256 byte codes mapped uniquely; real evaluator parity, exact words, T5 parity and file length verified"
    );
    Ok(())
}
