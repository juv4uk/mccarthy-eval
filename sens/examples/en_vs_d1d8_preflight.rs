//! #3113 — paired English/canonical-D1-D8 preflight for #3088.
//!
//! This executable proves that two source projections lower to the same
//! current exact-domain trace before any timing is allowed.

use sens::{lower_program, parse, parse_canonical_binary, CoreDomainIdentity, DomainIdentity, Expr, ExprKind};
use std::{env, fs, process::ExitCode};

fn domain_key(prefix: &str, identity: DomainIdentity) -> String {
    let width = identity.width();
    format!(
        "{prefix}:D{width}:{:0width$b}",
        identity.packed_bits(),
        width = width
    )
}

fn core_key(prefix: &str, identity: CoreDomainIdentity) -> String {
    let width = identity.width();
    format!(
        "{prefix}:D{width}:{:0width$b}",
        identity.packed_bits(),
        width = width
    )
}

fn trace_expr(expression: &Expr, trace: &mut Vec<String>, legacy: &mut bool) {
    match &expression.kind {
        ExprKind::DomainIdentity(identity) => trace.push(domain_key("ID", *identity)),
        ExprKind::DomainCall(identity, arguments) => {
            trace.push(core_key("CALL", *identity));
            for argument in arguments.iter() {
                trace_expr(argument, trace, legacy);
            }
        }
        ExprKind::Sid(_) => {
            *legacy = true;
            trace.push("LEGACY_BYTE_IDENTITY".to_owned());
        }
        ExprKind::Call(_, arguments) => {
            *legacy = true;
            trace.push("LEGACY_BYTE_CALL".to_owned());
            for argument in arguments.iter() {
                trace_expr(argument, trace, legacy);
            }
        }
        ExprKind::List(items) => {
            trace.push(format!("LIST:{}", items.len()));
            for item in items.iter() {
                trace_expr(item, trace, legacy);
            }
        }
        ExprKind::Pair(first, rest) => {
            trace.push("PAIR".to_owned());
            trace_expr(first, trace, legacy);
            trace_expr(rest, trace, legacy);
        }
        _ => trace.push(format!("LEAF:{:?}", &expression.kind)),
    }
}

fn lower_english(source: &str) -> Result<Vec<Expr>, String> {
    parse(source)
        .map(|parsed| lower_program(&parsed))
        .map_err(|error| format!("english parse/lower: {error}"))
}

fn lower_binary(source: &str) -> Result<Vec<Expr>, String> {
    parse_canonical_binary(source)
        .map(|parsed| lower_program(&parsed))
        .map_err(|error| format!("binary parse/lower: {error}"))
}

fn trace(program: &[Expr]) -> (Vec<String>, bool) {
    let mut observed = Vec::new();
    let mut legacy = false;
    for expression in program {
        trace_expr(expression, &mut observed, &mut legacy);
    }
    (observed, legacy)
}

fn compare_sources(english: &str, binary: &str, verbose: bool) -> Result<(), &'static str> {
    let english = lower_english(english).map_err(|_| "BLOCKED_PARSE")?;
    let binary = lower_binary(binary).map_err(|_| "BLOCKED_PARSE")?;
    let (english_trace, english_legacy) = trace(&english);
    let (binary_trace, binary_legacy) = trace(&binary);

    if english_legacy || binary_legacy {
        if verbose {
            println!("status\tBLOCKED_LEGACY_PATH");
        }
        return Err("BLOCKED_LEGACY_PATH");
    }

    if english_trace != binary_trace {
        if verbose {
            println!("status\tBLOCKED_TRACE_MISMATCH");
            for (index, item) in english_trace.iter().enumerate() {
                println!("english\t{index}\t{item}");
            }
            for (index, item) in binary_trace.iter().enumerate() {
                println!("binary\t{index}\t{item}");
            }
        }
        return Err("BLOCKED_TRACE_MISMATCH");
    }

    if verbose {
        println!("status\tPREFLIGHT_OK");
        println!("trace_nodes\t{}", english_trace.len());
        for (index, item) in english_trace.iter().enumerate() {
            println!("trace\t{index}\t{item}");
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: en_vs_d1d8_preflight ENGLISH.lisp CANONICAL.lisp");
        return ExitCode::from(2);
    }

    let english = match fs::read_to_string(&args[0]) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[0]);
            return ExitCode::from(2);
        }
    };
    let binary = match fs::read_to_string(&args[1]) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[1]);
            return ExitCode::from(2);
        }
    };

    match compare_sources(&english, &binary, true) {
        Ok(()) => ExitCode::SUCCESS,
        Err("BLOCKED_LEGACY_PATH") => ExitCode::from(3),
        Err("BLOCKED_TRACE_MISMATCH") => ExitCode::from(4),
        Err(_) => ExitCode::from(5),
    }
}
