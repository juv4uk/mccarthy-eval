use std::env;
use std::fs;

use cml::lower;
use cml::parser;
use cml::witness_bridge::{execute_x86_actual_with_metadata, WitnessBridgeError};
use cml::x86_freestanding::{CompileError, X86FreestandingBackend};

fn print_record(status: &str, value: &str) {
    let sanitized = value.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n");
    println!("{status}\t{sanitized}");
}

fn main() {
    let source_path = env::args()
        .nth(1)
        .unwrap_or_else(|| {
            eprintln!("usage: cml-witness-driver <lisp-source>");
            std::process::exit(2);
        });

    let source = match fs::read_to_string(&source_path) {
        Ok(source) => source,
        Err(error) => {
            print_record("error", &format!("read source: {error}"));
            std::process::exit(3);
        }
    };

    let expressions = match parser::parse(&source) {
        Ok(expressions) => expressions,
        Err(error) => {
            print_record("error", &format!("parse: {error:?}"));
            std::process::exit(4);
        }
    };

    let program = match lower::lower_program(&expressions) {
        Ok(program) => program,
        Err(error) => {
            print_record("error", &format!("lower: {error}"));
            std::process::exit(5);
        }
    };

    let compiled = match X86FreestandingBackend::new().compile_program_with_metadata(&program) {
        Ok(compiled) => compiled,
        Err(CompileError::UnsupportedVariant(node)) => {
            print_record("unsupported", node);
            return;
        }
        Err(error) => {
            print_record("error", &format!("compile: {error}"));
            std::process::exit(6);
        }
    };

    match execute_x86_actual_with_metadata(&compiled) {
        Ok(actual) => print_record("ok", &actual),
        Err(WitnessBridgeError::UnsupportedActual(word)) => {
            print_record("unsupported", &format!("target-word {word:#x}"));
        }
        Err(WitnessBridgeError::InvalidComposite(message)) => {
            print_record("error", &format!("invalid-composite: {message}"));
            std::process::exit(7);
        }
        Err(error) => {
            print_record("error", &error.to_string());
            std::process::exit(8);
        }
    }
}
