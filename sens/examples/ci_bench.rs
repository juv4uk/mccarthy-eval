//! #1433: швидкий бенчмарк для CI — один процес на одне навантаження.
//!
//! Міряється під valgrind (кількість інструкцій — відтворювана, на відміну
//! від часу на спільній машині). Бібліотека core НЕ завантажується:
//! навантаження користуються лише примітивами мови.
//!
//! SENS — це 8-бітна функція, а не текст. Тому програма у формі `sens`
//! виконується з двійкового вигляду (fasl), де функція займає рівно 1 байт;
//! текст `run.py` — лише людська поверхня, з якої двійковий вигляд кодується
//! заздалегідь (режим `encode`, не міряється). Англійська форма (`en`)
//! читається з тексту — це її людська поверхня.
//!
//! Human/English source is lowered exactly once at the frontend boundary:
//! parse -> lower_program. The evaluator never resolves human function spelling.
//! SENS executes from binary FASL, where Function8 is already the execution identity.
//! The benchmark therefore compares human-boundary parse+lower against binary decode,
//! while both execution paths run the same lowered binary identities.
//!
//!   ci_bench DIR NAME FORM [MODE]
//!     MODE = full (за замовчуванням) — завантажити й виконати, звірити відповідь;
//!            load — лише завантажити програму (розбір тексту або декодування fasl);
//!            ready — завантажити + виконати setup, але не call;
//!            repeat N — завантажити + setup один раз + виконати call N разів;
//!            encode — `sens`: закодувати текст у DIR/NAME-sens.{setup,call}.fasl.
//!   ci_bench DIR empty -     — лише створення сесії.
//!
//! Лише стабільний публічний API (`parse`, `fasl_encode_program`,
//! `fasl_decode_program`, `Session`, `eval_lowered_expressions`), щоб той самий
//! файл збирався й на попередньому коміті.

use sens::{
    eval_lowered_expressions, fasl_decode_program, fasl_encode_program, lower_program, parse,
    parse_exact_domain_source, Expr,
    Session,
};
use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let (dir, name, form) = (Path::new(&args[1]), &args[2], &args[3]);
    let mode = args.get(4).map(String::as_str).unwrap_or("full");
    let mut session = Session::default();
    if name == "empty" {
        return ExitCode::SUCCESS;
    }
    let file = |part: &str, ext: &str| dir.join(format!("{name}-{form}.{part}.{ext}"));
    let text = |part: &str| {
        fs::read_to_string(file(part, "lisp")).unwrap_or_else(|e| panic!("{name}-{form}.{part}: {e}"))
    };

    if mode == "encode" {
        // Текст форми `sens` -- це точні коди, не людські імена й не числа:
        // читаємо їх точним читачем, інакше голий код на кшталт 01011
        // став би числом 11, а не D5 DIFFERENCE.
        for part in ["setup", "call"] {
            let program = parse_exact_domain_source(&text(part)).expect("exact sens text parses");
            fs::write(file(part, "fasl"), fasl_encode_program(&program, &[0u8; 32]))
                .expect("fasl written");
        }
        return ExitCode::SUCCESS;
    }

    let load = |part: &str| -> Vec<Expr> {
        if form == "sens" {
            let bytes = fs::read(file(part, "fasl"))
                .unwrap_or_else(|e| panic!("{name}-sens.{part}.fasl (run encode first): {e}"));
            fasl_decode_program(&bytes).expect("fasl decodes").0
        } else if form == "sens-text" {
            // SENS-коди як текст: точний читач, без lower (коди вже виконувані).
            parse_exact_domain_source(&text(part)).expect("exact sens text parses")
        } else {
            let parsed = parse(&text(part)).expect("text parses");
            lower_program(&parsed)
        }
    };
    let setup = load("setup");
    let call = load("call");
    if mode == "load" {
        std::hint::black_box((&setup, &call));
        return ExitCode::SUCCESS;
    }

    let expected = fs::read_to_string(dir.join(format!("{name}.expected")))
        .expect("expected answer")
        .trim()
        .to_owned();
    if let Err(error) = eval_lowered_expressions(&setup, &mut session) {
        eprintln!("{name}/{form}: setup failed: {error}");
        return ExitCode::from(2);
    }
    if mode == "ready" {
        return ExitCode::SUCCESS;
    }
    if mode == "repeat" {
        let repetitions = args
            .get(5)
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .expect("repeat mode requires positive N");
        for _ in 0..repetitions {
            match eval_lowered_expressions(&call, &mut session) {
                Ok(result) => {
                    std::hint::black_box(result.value);
                }
                Err(error) => {
                    eprintln!("{name}/{form}: repeated call failed: {error}");
                    return ExitCode::from(4);
                }
            }
        }
        return ExitCode::SUCCESS;
    }
    match eval_lowered_expressions(&call, &mut session) {
        Ok(result) if result.value.to_string() == expected => ExitCode::SUCCESS,
        Ok(result) => {
            eprintln!("{name}/{form}: expected {expected}, got {}", result.value);
            ExitCode::from(3)
        }
        Err(error) => {
            eprintln!("{name}/{form}: {error}");
            ExitCode::from(4)
        }
    }
}
