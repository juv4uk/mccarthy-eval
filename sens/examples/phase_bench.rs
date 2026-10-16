//! Зріз за шарами: MODE = session | parse | lower | full.
//! Вартість шару — різниця інструкцій сусідніх режимів (valgrind).
use sens::{eval_lowered_expressions, lower_program, parse, Session};
use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let (dir, name, form, mode) = (Path::new(&args[1]), &args[2], &args[3], args[4].as_str());
    let mut session = Session::default();
    let read = |file: String| fs::read_to_string(dir.join(&file)).unwrap();
    let setup_src = read(format!("{name}-{form}.setup.lisp"));
    let call_src = read(format!("{name}-{form}.call.lisp"));
    let expected = read(format!("{name}.expected")).trim().to_owned();
    if mode == "session" {
        return ExitCode::SUCCESS;
    }
    let setup = parse(&setup_src).expect("setup parses");
    let call = parse(&call_src).expect("call parses");
    if mode == "parse" {
        std::hint::black_box((&setup, &call));
        return ExitCode::SUCCESS;
    }
    let setup = lower_program(&setup);
    let call = lower_program(&call);
    if mode == "lower" {
        std::hint::black_box((&setup, &call));
        return ExitCode::SUCCESS;
    }
    eval_lowered_expressions(&setup, &mut session).expect("setup evaluates");
    match eval_lowered_expressions(&call, &mut session) {
        Ok(r) if r.value.to_string() == expected => ExitCode::SUCCESS,
        Ok(r) => { eprintln!("{name}/{form}: expected {expected}, got {}", r.value); ExitCode::from(3) }
        Err(e) => { eprintln!("{name}/{form}: {e}"); ExitCode::from(4) }
    }
}
