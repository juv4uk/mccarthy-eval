//! Дослідницький бенчмарк: Core1 проти Meta-Eval на одному Rust-хості.
//! Не змінює семантику SENS і не є доказом переносимості або роботи GC.
//! cargo run --locked --profile ci-meta -p sens --example bench_core1_meta -- --samples 25 --batch 5 --warmup 10
use sens::{eval_lowered_expressions, eval_program, lower_program, parse, Session};
use std::hint::black_box;
use std::time::Instant;

const CORE1_SOURCE: &str = include_str!("../../../lib/core1.lisp");
#[derive(Clone, Copy)]
struct Case { id: &'static str, quoted_form: &'static str }
const CASES: &[Case] = &[
    Case { id: "quote-atom", quoted_form: "(quote (quote a))" },
    Case { id: "quote-list", quoted_form: "(quote (quote (a b c)))" },
    Case { id: "car", quoted_form: "(quote (car (quote (a b c))))" },
    Case { id: "cdr", quoted_form: "(quote (cdr (quote (a b c))))" },
    Case { id: "cons", quoted_form: "(quote (cons (quote a) (quote (b c))))" },
    Case { id: "two-selectors", quoted_form: "(quote (car (cdr (quote (a b c)))))" },
    Case { id: "atom", quoted_form: "(quote (atom (quote a)))" },
    Case { id: "eq", quoted_form: "(quote (eq (quote a) (quote a)))" },
    Case { id: "cond", quoted_form: "(quote (cond ((eq (quote a) (quote a)) (quote yes)) ((eq (quote a) (quote b)) (quote no))))" },
    Case { id: "lambda", quoted_form: "(quote ((lambda (x) (cons x (quote ()))) (quote a)))" },
];

fn opt_number(args: &[String], key: &str, default: usize) -> usize {
    args.windows(2).find(|pair| pair[0] == key)
        .map(|pair| pair[1].parse::<usize>().expect("expected positive integer argument"))
        .unwrap_or(default)
}
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 { sorted[n / 2] }
    else { (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0 }
}
fn p95(sorted: &[f64]) -> f64 {
    let idx = ((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
    sorted[idx.min(sorted.len() - 1)]
}
fn run_once(session: &mut Session, ast: &[sens::Expr]) -> String {
    eval_lowered_expressions(ast, session)
        .unwrap_or_else(|err| panic!("evaluator failed: {err}")).value.to_string()
}
fn measure(session: &mut Session, ast: &[sens::Expr], samples: usize, batch: usize, warmup: usize) -> (f64, f64) {
    for _ in 0..warmup {
        black_box(eval_lowered_expressions(ast, session).expect("warmup evaluator failure"));
    }
    let mut observations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(eval_lowered_expressions(ast, session).expect("timed evaluator failure"));
        }
        observations.push(start.elapsed().as_secs_f64() * 1_000_000.0 / batch as f64);
    }
    observations.sort_by(f64::total_cmp);
    (median(&observations), p95(&observations))
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let samples = opt_number(&args, "--samples", 25);
    let batch = opt_number(&args, "--batch", 5);
    let warmup = opt_number(&args, "--warmup", 10);
    assert!(samples > 0 && batch > 0, "samples and batch must be nonzero");

    let mut core_session = Session::default();
    let core_boot_start = Instant::now();
    sens::load_core_library(&mut core_session).expect("core library for Core1");
    eval_program(CORE1_SOURCE, &mut core_session)
        .expect("Core1 bootstrap failed; report BLOCKED, not benchmark numbers");
    let core_boot_us = core_boot_start.elapsed().as_micros();

    let mut meta_session = Session::default();
    let meta_boot_start = Instant::now();
    sens::load_core_library(&mut meta_session).expect("core library for Meta-Eval");
    sens::load_meta_evaluator_library(&mut meta_session)
        .expect("Meta-Eval bootstrap failed; report BLOCKED, not benchmark numbers");
    let meta_boot_us = meta_boot_start.elapsed().as_micros();

    println!("benchmark=CORE1_VS_META_EVAL_SAME_RUST_HOST");
    println!("status=RESEARCH_NON_GATE_NOT_SEMANTIC_AUTHORITY");
    println!("samples={samples} batch={batch} warmup={warmup}");
    println!("core1_bootstrap_us={core_boot_us} meta_bootstrap_us={meta_boot_us}");
    println!("case,core1_p50_us,core1_p95_us,meta_p50_us,meta_p95_us,meta_div_core1,expected_value");

    let mut compared = 0usize;
    let mut different = 0usize;
    for (i, case) in CASES.iter().enumerate() {
        let c_source = format!("(C1-EVAL {} (quote ()) (quote ()))", case.quoted_form);
        let m_source = format!("(my-eval {} (quote ()))", case.quoted_form);
        let c_ast = lower_program(&parse(&c_source).expect("Core1 case parse"));
        let m_ast = lower_program(&parse(&m_source).expect("Meta case parse"));
        let c_out = run_once(&mut core_session, &c_ast);
        let m_out = run_once(&mut meta_session, &m_ast);
        if c_out != m_out {
            different += 1;
            eprintln!("BLOCKED_DIFFERENT_SEMANTICS case={} core1={:?} meta={:?}", case.id, c_out, m_out);
            continue;
        }
        let (c, m) = if i % 2 == 0 {
            (measure(&mut core_session, &c_ast, samples, batch, warmup),
             measure(&mut meta_session, &m_ast, samples, batch, warmup))
        } else {
            let m = measure(&mut meta_session, &m_ast, samples, batch, warmup);
            let c = measure(&mut core_session, &c_ast, samples, batch, warmup);
            (c, m)
        };
        compared += 1;
        println!("{},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
            case.id, c.0, c.1, m.0, m.1, m.0 / c.0, c_out);
    }
    println!("summary=COMPARED:{compared};SEMANTIC_MISMATCH:{different};TOTAL:{}", CASES.len());
    if compared == 0 {
        eprintln!("BLOCKED_NO_COMPARABLE_CASES — жодного валідного виміру");
        std::process::exit(2);
    }
}