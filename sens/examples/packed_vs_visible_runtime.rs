//! Measured execution pipeline for real binary bytes vs visible D2.
//! No historical SID8 routing, invented domain law, or text conversion.
//! Framing word widths are caller supplied and excluded from byte claims.
use sens::{
    eval_lowered_expressions, eval_parsed_expressions, lower_program,
    pack_binary_source_tokens, parse_binary_source_words, parse_canonical_binary,
    parse_canonical_packed_words, Session, Value,
};
use std::{hint::black_box, time::Instant};

const SCHEMA: &str = "sens-packed-visible-runtime/v1";

fn fingerprint(value: &Value, output: &[String]) -> String {
    format!("{value:?}|{output:?}")
}

fn per_program<F: FnMut()>(iterations: usize, f: &mut F) -> f64 {
    let started = Instant::now();
    for _ in 0..iterations {
        f();
    }
    started.elapsed().as_nanos() as f64 / iterations as f64
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}

fn paired<F: FnMut(), G: FnMut()>(
    mut visible: F,
    mut packed: G,
    iterations: usize,
    samples: usize,
) -> (Vec<f64>, Vec<f64>) {
    // Warm both paths before samples, and reverse the measurement order on
    // alternate batches to reduce persistent order/caching bias.
    for _ in 0..24 {
        visible();
        packed();
    }
    let mut a = Vec::with_capacity(samples);
    let mut b = Vec::with_capacity(samples);
    for rep in 0..samples {
        if rep % 2 == 0 {
            a.push(per_program(iterations, &mut visible));
            b.push(per_program(iterations, &mut packed));
        } else {
            b.push(per_program(iterations, &mut packed));
            a.push(per_program(iterations, &mut visible));
        }
    }
    (a, b)
}

fn measured_baseline<F: FnMut()>(mut f: F, iterations: usize, samples: usize) -> f64 {
    for _ in 0..24 {
        f();
    }
    let timings = (0..samples)
        .map(|_| per_program(iterations, &mut f))
        .collect::<Vec<_>>();
    median(&timings)
}

fn emit(
    case: &str,
    phase: &str,
    forms: usize,
    samples: usize,
    iterations: usize,
    semantic_bits: usize,
    packed_bytes: usize,
    text_bytes: usize,
    width_entries: usize,
    execution_baseline_ns: f64,
    visible_samples: &[f64],
    packed_samples: &[f64],
) -> Result<(), String> {
    let a = median(visible_samples);
    let b = median(packed_samples);
    if !(a > 0.0 && b > 0.0 && a.is_finite() && b.is_finite()) {
        return Err(format!("{case}/{phase}: no valid wall-clock samples"));
    }
    println!(
        "{{\"schema\":\"{SCHEMA}\",\"case\":\"{case}\",\"phase\":\"{phase}\",\"forms\":{forms},\"iterations_per_sample\":{iterations},\"samples\":{samples},\"visible_median_ns\":{a:.2},\"packed_median_ns\":{b:.2},\"ratio_visible_over_packed\":{:.5},\"execute_only_baseline_ns\":{execution_baseline_ns:.2},\"semantic_bits\":{semantic_bits},\"packed_payload_bytes\":{packed_bytes},\"visible_text_bytes\":{text_bytes},\"external_width_entries\":{width_entries},\"visible_samples_ns\":{visible_samples:?},\"packed_samples_ns\":{packed_samples:?},\"oracle\":\"same-evaluated-value-output\",\"width_metadata\":\"external-not-measured\"}}",
        a / b,
    );
    Ok(())
}

fn benchmark(case: &str, source: &str, forms: usize, samples: usize) -> Result<(), String> {
    let text = std::iter::repeat(source).take(forms).collect::<Vec<_>>().join("\n");
    let tokens = parse_binary_source_words(&text)
        .map_err(|e| format!("{case}: source lex: {e:?}"))?;
    let widths = tokens.iter().map(|t| t.word.width()).collect::<Vec<_>>();
    let physical = pack_binary_source_tokens(&tokens);
    let from_text = parse_canonical_binary(&text)
        .map_err(|e| format!("{case}: visible D2: {e:?}"))?;
    let from_bytes = parse_canonical_packed_words(&physical, &widths)
        .map_err(|e| format!("{case}: packed D2: {e:?}"))?;
    if from_text.len() != forms || from_bytes.len() != forms {
        return Err(format!("{case}: incomplete D2 corpus; refuse timing"));
    }

    // Independent output parity must hold BEFORE any measured sample. Evaluated
    // results may share empty-list spelling, so distinguish exact D1 here.
    let left = eval_parsed_expressions(&from_text, &mut Session::default())
        .map_err(|e| format!("{case}: visible eval: {e:?}"))?;
    let right = eval_parsed_expressions(&from_bytes, &mut Session::default())
        .map_err(|e| format!("{case}: physical eval: {e:?}"))?;
    if fingerprint(&left.value, &left.output) != fingerprint(&right.value, &right.output) {
        return Err(format!("{case}: mismatched program result: BLOCK"));
    }
    if case.starts_with("d1-") {
        if left.value.as_predicate_bit() != Some(true) {
            return Err(format!("{case}: expected exact D1:1, not truthiness"));
        }
    } else if !matches!(left.value, Value::Nil) {
        return Err(format!("{case}: expected structural EMPTY, not a D1 bit"));
    }

    let iterations = (4096 / forms).clamp(16, 512);
    let lowered = lower_program(&from_text);
    let mut already_lowered_session = Session::default();
    let eval_only = measured_baseline(
        || {
            let out = eval_lowered_expressions(black_box(&lowered), &mut already_lowered_session)
                .expect("preflighted exact-domain execute");
            black_box(out.value);
        },
        iterations,
        samples,
    );

    let mut session_text = Session::default();
    let mut session_bytes = Session::default();
    let (warm_a, warm_b) = paired(
        || {
            let ast = parse_canonical_binary(black_box(&text))
                .expect("preflighted visible D2");
            let out = eval_parsed_expressions(&ast, &mut session_text)
                .expect("preflighted visible execution");
            black_box(out.value);
        },
        || {
            let ast = parse_canonical_packed_words(black_box(&physical), black_box(&widths))
                .expect("preflighted packed D2");
            let out = eval_parsed_expressions(&ast, &mut session_bytes)
                .expect("preflighted packed execution");
            black_box(out.value);
        },
        iterations,
        samples,
    );
    emit(case, "warm-session-parse-lower-eval", forms, samples, iterations,
         physical.bit_len(), physical.byte_len(), text.len(), widths.len(),
         eval_only, &warm_a, &warm_b)?;

    let (cold_a, cold_b) = paired(
        || {
            let mut session = Session::default();
            let ast = parse_canonical_binary(black_box(&text))
                .expect("preflighted visible D2");
            let out = eval_parsed_expressions(&ast, &mut session)
                .expect("preflighted visible execution");
            black_box(out.value);
        },
        || {
            let mut session = Session::default();
            let ast = parse_canonical_packed_words(black_box(&physical), black_box(&widths))
                .expect("preflighted packed D2");
            let out = eval_parsed_expressions(&ast, &mut session)
                .expect("preflighted packed execution");
            black_box(out.value);
        },
        iterations,
        samples,
    );
    emit(case, "fresh-session-parse-lower-eval", forms, samples, iterations,
         physical.bit_len(), physical.byte_len(), text.len(), widths.len(),
         eval_only, &cold_a, &cold_b)
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let samples = if args.len() == 3 && args[1] == "--samples" {
        args[2].parse::<usize>().unwrap_or(0)
    } else if args.len() == 1 {
        9
    } else {
        0
    };
    if !(3..=99).contains(&samples) || samples % 2 == 0 {
        eprintln!("usage: packed_vs_visible_runtime [--samples ODD_NUMBER_3_TO_99]");
        std::process::exit(2);
    }

    let cases = [
        ("quote-empty-1", "10 001 00 000 01", 1),
        ("quote-empty-64", "10 001 00 000 01", 64),
        ("quote-empty-256", "10 001 00 000 01", 256),
        ("d1-one-64", "1", 64),
    ];
    for (name, source, forms) in cases {
        if let Err(err) = benchmark(name, source, forms, samples) {
            eprintln!("BENCHMARK BLOCKED: {err}");
            std::process::exit(2);
        }
    }
}
