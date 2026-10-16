//! Objective paired ingest benchmark: visible 0/1 source vs packed source bytes.
//! This is not a semantic authority and not a self-describing file benchmark.
//! Width boundaries are supplied to the packed reader out of band.
use sens::{
    pack_binary_source_tokens, parse_binary_source_words, parse_canonical_binary,
    parse_canonical_packed_words, Expr, ExprKind,
};
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

const SCHEMA: &str = "sens-packed-visible-ingest/v1";

fn shape(expr: &Expr, output: &mut String) {
    match &expr.kind {
        ExprKind::DomainIdentity(id) => {
            write!(output, "D{}:{:b};", id.width(), id.packed_bits()).unwrap();
        }
        ExprKind::List(items) => {
            output.push('[');
            for item in items.iter() {
                shape(item, output);
            }
            output.push(']');
        }
        ExprKind::Pair(head, tail) => {
            output.push('(');
            shape(head, output);
            output.push('.');
            shape(tail, output);
            output.push(')');
        }
        other => {
            write!(output, "{other:?}").unwrap();
        }
    }
}

fn fingerprint(forms: &[Expr]) -> String {
    let mut output = String::new();
    for form in forms {
        shape(form, &mut output);
        output.push('|');
    }
    output
}

fn elapsed_per_program<F, T>(iterations: usize, f: &mut F) -> f64
where
    F: FnMut() -> T,
{
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(f());
    }
    (start.elapsed().as_nanos() as f64) / iterations as f64
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    sorted[sorted.len() / 2]
}

fn benchmark(case: &str, form: &str, forms: usize, samples: usize) -> Result<(), String> {
    let visible = std::iter::repeat_n(form, forms)
        .collect::<Vec<_>>()
        .join("\n");
    let tokens = parse_binary_source_words(&visible)
        .map_err(|e| format!("{case}: source lexing: {e:?}"))?;
    let widths: Vec<usize> = tokens.iter().map(|token| token.word.width()).collect();
    let packed = pack_binary_source_tokens(&tokens);
    let visual_ast = parse_canonical_binary(&visible)
        .map_err(|e| format!("{case}: visible D2: {e:?}"))?;
    let physical_ast = parse_canonical_packed_words(&packed, &widths)
        .map_err(|e| format!("{case}: packed D2: {e:?}"))?;
    if fingerprint(&visual_ast) != fingerprint(&physical_ast) {
        return Err(format!("{case}: D2/domain AST parity failed: no performance claim"));
    }
    if visual_ast.len() != forms || physical_ast.len() != forms {
        return Err(format!("{case}: expected {forms} complete D2 forms"));
    }

    // Same AST grammar and compiled binary. Packed-width boundaries are already
    // known and *not* included in the timed ingest; report their count instead.
    // Both closures allocate their parsed AST on each measured invocation.
    let mut visible_read = || parse_canonical_binary(black_box(&visible))
        .expect("preflighted visible input");
    let mut packed_read = || parse_canonical_packed_words(
        black_box(&packed),
        black_box(&widths),
    ).expect("preflighted packed input");

    let iterations = (16_384 / forms).clamp(32, 2_048);
    for _ in 0..32 {
        black_box(visible_read());
        black_box(packed_read());
    }
    let mut visible_ns = Vec::with_capacity(samples);
    let mut packed_ns = Vec::with_capacity(samples);
    for sample in 0..samples {
        if sample.is_multiple_of(2) {
            visible_ns.push(elapsed_per_program(iterations, &mut visible_read));
            packed_ns.push(elapsed_per_program(iterations, &mut packed_read));
        } else {
            packed_ns.push(elapsed_per_program(iterations, &mut packed_read));
            visible_ns.push(elapsed_per_program(iterations, &mut visible_read));
        }
    }

    let v = median(&visible_ns);
    let p = median(&packed_ns);
    if !(v.is_finite() && p.is_finite() && v > 0.0 && p > 0.0) {
        return Err(format!("{case}: invalid timing, refuse a speed claim"));
    }
    println!(
        "{{\"schema\":\"{SCHEMA}\",\"case\":\"{case}\",\"forms\":{forms},\"iterations_per_sample\":{iterations},\"samples\":{samples},\"visible_median_ns\":{v:.2},\"packed_median_ns\":{p:.2},\"ratio_visible_over_packed\":{:.5},\"visible_source_bytes\":{},\"packed_payload_bytes\":{},\"semantic_payload_bits\":{},\"width_schedule_entries\":{},\"visible_samples_ns\":{:?},\"packed_samples_ns\":{:?},\"preflight\":\"same-D2-domain-AST\",\"boundary_policy\":\"caller-supplied-excluded-from-payload\"}}",
        v / p,
        visible.len(),
        packed.byte_len(),
        packed.bit_len(),
        widths.len(),
        visible_ns,
        packed_ns,
    );
    Ok(())
}

fn main() {
    let mut samples = 9usize;
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--samples" {
        samples = args[2].parse().unwrap_or(0);
    } else if args.len() != 1 {
        eprintln!("usage: packed_vs_visible_bench [--samples 3..99]");
        std::process::exit(2);
    }
    if !(3..=99).contains(&samples) || samples.is_multiple_of(2) {
        eprintln!("samples must be odd, between 3 and 99");
        std::process::exit(2);
    }
    let cases = [
        ("quote-empty-1", "10 001 00 000 01", 1),
        ("quote-empty-64", "10 001 00 000 01", 64),
        ("quote-empty-256", "10 001 00 000 01", 256),
        ("d7-data-64", "10 1000001 00 1000010 01", 64),
        ("w9-data-64", "10 100000001 00 00000001 01", 64),
    ];
    for (name, source, count) in cases {
        if let Err(error) = benchmark(name, source, count, samples) {
            eprintln!("BLOCKED: {error}");
            std::process::exit(2);
        }
    }
}
