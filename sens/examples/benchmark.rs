use sens::{eval_program, load_core_library, parse, Session};
use std::{fs, hint::black_box, time::Instant};

const CASES: &[(&str, &str)] = &[
    ("arithmetic", "benchmarks/arithmetic.lisp"),
    ("lists", "benchmarks/lists.lisp"),
    ("recursion", "benchmarks/recursion.lisp"),
    ("closures", "benchmarks/closures.lisp"),
];

fn measure(iterations: usize, mut operation: impl FnMut()) -> f64 {
    for _ in 0..50 {
        operation();
    }
    let started = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    started.elapsed().as_nanos() as f64 / iterations as f64
}

// ── warm-session section ─────────────────────────────────────────────
// The fresh-session cases above pay core.my parsing on every operation;
// these measure steady-state interpreter throughput against the two
// hottest shapes from docs/OPTIMIZATION-ANALYSIS-VYASA.md: exact
// rational chains (LCM denominator growth drives bignum cost) and
// vector mutation loops. Machine-specific — compare same-machine runs.

const WARM_SETUP: &str = r#"
(def rat-loop
  (lambda (n acc)
    (cond
      ((= n 0) acc)
      (t (rat-loop (- n 1)
                   (+ acc (/ (* n n) (+ (* n 3) 1))))))))
(def vfill
  (lambda (v n)
    (cond
      ((= n 0) v)
      (t (cons (vector-set! v (mod n 8) n)
               (vfill v (- n 1)))))))
(def v0 (make-vector 8))

; fixnum-loop mirrors rat-loop's exact shape (same recursion depth, same
; number of +/* calls per iteration, same cond/call dispatch overhead) but
; every intermediate value stays an integer -- no division, so the
; accumulator never becomes a non-trivial rational. This isolates the cost
; of exact-rational normalization (gcd reduction, growing numerator/
; denominator) from ordinary recursive-call/dispatch overhead: the delta
; between fixnum-loop and rat-loop at the same n is (approximately) the
; rational-arithmetic tax alone, not a confound of loop shape.
(def fixnum-loop
  (lambda (n acc)
    (cond
      ((= n 0) acc)
      (t (fixnum-loop (- n 1)
                      (+ acc (* n n)))))))

; symbol-lookup-shallow/-deep isolate environment-chain-walk cost.
; sens's Environment is a linked parent-frame chain (see
; crates/sens/src/environment.rs), so looking up a name bound near the
; global root from deep inside nested lambda calls must walk every
; intervening frame. Both loops do identical arithmetic and identical
; recursion depth; the only difference is how many lexical frames separate
; the reference site from `far-away`'s binding. The wrapper functions exist
; purely to add lexical nesting depth without changing what each recursive
; step computes, so any per-call delta between shallow and deep is
; attributable to environment-chain length, not to the arithmetic itself.
(def far-away 7)
(def lookup-shallow-loop
  (lambda (n acc)
    (cond
      ((= n 0) acc)
      (t (lookup-shallow-loop (- n 1) (+ acc far-away))))))
(def lookup-wrap-1 (lambda (n acc) ((lambda (n acc) (lookup-wrap-2 n acc)) n acc)))
(def lookup-wrap-2 (lambda (n acc) ((lambda (n acc) (lookup-wrap-3 n acc)) n acc)))
(def lookup-wrap-3 (lambda (n acc) ((lambda (n acc) (lookup-wrap-4 n acc)) n acc)))
(def lookup-wrap-4 (lambda (n acc) ((lambda (n acc) (lookup-deep-loop n acc)) n acc)))
(def lookup-deep-loop
  (lambda (n acc)
    (cond
      ((= n 0) acc)
      (t (lookup-wrap-1 (- n 1) (+ acc far-away))))))
"#;

fn warm(name: &str, source: &str, iterations: usize) {
    let mut session = Session::default();
    load_core_library(&mut session).expect("canonical macro + core bootstrap should preload");
    eval_program(WARM_SETUP, &mut session).expect("warm setup");
    for _ in 0..3 {
        black_box(eval_program(source, &mut session).expect("warm-up"));
    }
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(eval_program(black_box(source), &mut session).expect("hot path"));
    }
    let ns = started.elapsed().as_nanos() as f64 / iterations as f64;
    println!("BENCH_RESULT\twarm\t{name}\t{ns:.2}");
}

fn main() {
    // `warm/vector-fill-500` (below) recurses deep enough to overflow a
    // 1 MiB thread stack (Windows' default main-thread size) even though
    // it comfortably fits an 8 MiB stack (a typical Linux default) --
    // confirmed directly: docs/benchmarks.md §5. Running the whole
    // benchmark on an explicit larger-stack worker thread makes this
    // dev tool's results reproducible across platforms without changing
    // eval_program's own non-tail-recursive call path (a separate,
    // larger fix tracked as a real finding, not applied here).
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(run_benchmarks)
        .expect("spawn benchmark worker thread");
    handle.join().expect("benchmark worker thread panicked");
}

fn run_benchmarks() {
    let iterations = std::env::var("MY_LISP_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1_000);
    let parser_source = fs::read_to_string("benchmarks/parser.lisp").expect("read parser benchmark");
    let parser_ns = measure(iterations, || {
        black_box(parse(black_box(&parser_source)).expect("parse benchmark"));
    });
    println!("BENCH_RESULT\trust\tparser\t{parser_ns:.2}");

    for (name, path) in CASES {
        let source = fs::read_to_string(path).expect("read evaluation benchmark");
        let ns = measure(iterations, || {
            let mut session = Session::default();
            black_box(eval_program(black_box(&source), &mut session).expect("evaluate benchmark"));
        });
        println!("BENCH_RESULT\trust\t{name}\t{ns:.2}");
    }

    // ── warm-session steady state ──
    let rat_n = std::env::var("MY_LISP_RAT_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100usize);
    let rat = format!("(rat-loop {rat_n} 0)");
    warm("rational-chain-100", &rat, 50.max(iterations / 20));

    // fixnum-loop vs rat-loop, same n, same shape: isolates exact-rational
    // normalization overhead from recursive-call/dispatch cost. Any large
    // gap here is the "rational tax" documented in docs/benchmarks.md,
    // now given a same-shape integer control instead of just an absolute
    // number.
    let fixnum = format!("(fixnum-loop {rat_n} 0)");
    warm("fixnum-loop-100", &fixnum, iterations.min(200));

    // symbol-lookup-shallow vs symbol-lookup-deep, same n, same arithmetic:
    // isolates environment-chain-walk cost for a name bound far from the
    // reference site (4 extra lexical frames between lookup site and
    // binding). Any gap here characterizes the real cost of sens's
    // linked-parent-frame Environment design, not measured before.
    let lookup_n = rat_n;
    let shallow = format!("(lookup-shallow-loop {lookup_n} 0)");
    warm("symbol-lookup-shallow", &shallow, iterations.min(200));
    let deep = format!("(lookup-deep-loop {lookup_n} 0)");
    warm("symbol-lookup-deep", &deep, iterations.min(200));

    let vecs = "(cons (vector-set! v0 (mod 7 8) 42) (vfill v0 500))";
    warm("vector-fill-500", vecs, iterations.min(200));
}
