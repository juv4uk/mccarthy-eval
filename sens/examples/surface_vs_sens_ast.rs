//! #1413: англійський Lisp проти SENS усередині одного процесу.
//!
//! Інший підхід, ніж `benchmarks/sens-surface/run.py` (цілий процес):
//! тут старт і завантаження core виключені, а виклик розбирається ОДИН
//! раз — міряється лише обчислення вже готового AST. Окремо міряється
//! вартість самого parse.
//!
//! Вхід — каталог, записаний `run.py --emit DIR`: для кожного
//! навантаження `<name>-<form>.setup.lisp`, `<name>-<form>.call.lisp`,
//! `<name>.expected`.
//!
//! `cargo run --release -p sens --example surface_vs_sens_ast -- DIR [SAMPLES] [TARGET_MS]`

use sens::{eval_parsed_expressions, eval_program, load_core_library, parse, Session};
use std::{env, fs, hint::black_box, path::Path, time::Instant};

const FORMS: [&str; 2] = ["en", "sens"];

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[xs.len() / 2]
}

fn read(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = Path::new(args.get(1).expect("каталог з run.py --emit"));
    let samples: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(9);
    let target_ms: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(300.0);

    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| {
            let n = e.ok()?.file_name().into_string().ok()?;
            n.strip_suffix(".expected").map(str::to_owned)
        })
        .collect();
    names.sort();

    println!("AST\tworkload\tform\teval_ns_median\teval_ns_min\teval_ns_max\tparse_ns_median\titers");
    for name in &names {
        let expected = read(dir, &format!("{name}.expected")).trim().to_owned();
        let mut sessions = Vec::new();
        let mut calls = Vec::new();
        let mut sources = Vec::new();
        for form in FORMS {
            let setup = read(dir, &format!("{name}-{form}.setup.lisp"));
            let call = read(dir, &format!("{name}-{form}.call.lisp"));
            let mut session = Session::default();
            load_core_library(&mut session).expect("core loads");
            eval_program(&setup, &mut session).expect("setup evaluates");
            let forms = parse(&call).expect("call parses");
            // Правильність — до будь-якого заміру.
            let got = eval_parsed_expressions(&forms, &mut session)
                .expect("call evaluates")
                .value
                .to_string();
            assert_eq!(got, expected, "{name}/{form}: неправильна відповідь");
            sessions.push(session);
            calls.push(forms);
            sources.push(setup + &call);
        }

        // Калібрування: скільки викликів дає ~TARGET_MS на семпл.
        let started = Instant::now();
        black_box(eval_parsed_expressions(&calls[1], &mut sessions[1]).unwrap());
        let one_ms = started.elapsed().as_secs_f64() * 1e3;
        let iters = ((target_ms / one_ms.max(1e-3)).ceil() as usize).max(1);

        // Семпли по черзі en/sens, порядок чергується між семплами.
        let mut per_form: Vec<Vec<f64>> = vec![Vec::new(), Vec::new()];
        for s in 0..samples {
            let order: [usize; 2] = if s % 2 == 0 { [0, 1] } else { [1, 0] };
            for &i in &order {
                let started = Instant::now();
                for _ in 0..iters {
                    black_box(eval_parsed_expressions(&calls[i], &mut sessions[i]).unwrap());
                }
                per_form[i].push(started.elapsed().as_nanos() as f64 / iters as f64);
            }
        }

        // Вартість parse повного тексту програми (setup + call).
        let mut parse_ns = Vec::new();
        for source in &sources {
            let reps = 200;
            let mut xs = Vec::new();
            for _ in 0..samples {
                let started = Instant::now();
                for _ in 0..reps {
                    black_box(parse(black_box(source)).unwrap());
                }
                xs.push(started.elapsed().as_nanos() as f64 / reps as f64);
            }
            parse_ns.push(median(xs));
        }

        for (i, form) in FORMS.iter().enumerate() {
            let xs = &per_form[i];
            let min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = xs.iter().cloned().fold(0.0, f64::max);
            println!(
                "AST\t{name}\t{form}\t{:.0}\t{min:.0}\t{max:.0}\t{:.0}\t{iters}",
                median(xs.clone()),
                parse_ns[i]
            );
        }
        let ratios: Vec<f64> = per_form[0]
            .iter()
            .zip(&per_form[1])
            .map(|(en, sens)| en / sens)
            .collect();
        let rmin = ratios.iter().cloned().fold(f64::INFINITY, f64::min);
        let rmax = ratios.iter().cloned().fold(0.0, f64::max);
        println!(
            "RATIO\t{name}\ten/sens\t{:.3}\t{rmin:.3}\t{rmax:.3}",
            median(ratios.clone())
        );
    }
}
