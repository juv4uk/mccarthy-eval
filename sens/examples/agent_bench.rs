//! Бенчмарк «агентських повідомлень»: агент отримує потік маленьких різних
//! програм байтами й кожну одразу виконує в одній довгоживучій сесії.
//!
//! Файл повідомлень — записи `u32 LE довжина + байти`. Форма `sens` — двійковий
//! fasl (функція = 1 байт); `wire` — компактний формат обміну (SENS wire);
//! форма `en` — англійський текст, який розбирається.
//!
//!   agent_bench encode FORMAT IN_TEXT_RECORDS OUT_RECORDS   (FORMAT = fasl | wire)
//!   agent_bench run FORM RECORDS MODE [EXPECTED]
//!     FORM = sens | wire | en
//!     MODE = base   — лише прочитати файл і створити сесію (база, яку віднімають);
//!            decode — прочитати + декодувати/розібрати кожне повідомлення;
//!            full   — прочитати + декодувати + виконати кожне повідомлення.
//!     EXPECTED — файл відповідей по рядку на повідомлення; з ним `full`
//!                звіряє кожну відповідь (неправильна — код 3).

use sens::{
    eval_lowered_expressions, fasl_decode_program, fasl_encode_program, parse, wire_decode_program,
    wire_encode_program, Expr, Session,
};
use std::{env, fs, process::ExitCode};

fn records(bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let len =
            u32::from_le_bytes(bytes[at..at + 4].try_into().expect("довжина запису")) as usize;
        out.push(&bytes[at + 4..at + 4 + len]);
        at += 4 + len;
    }
    out
}

fn load(form: &str, message: &[u8]) -> Vec<Expr> {
    if form == "sens" {
        fasl_decode_program(message).expect("fasl decodes").0
    } else if form == "wire" {
        wire_decode_program(message).expect("wire decodes")
    } else {
        parse(std::str::from_utf8(message).expect("utf-8 text")).expect("text parses")
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args[1] == "encode" {
        let wire = args[2] == "wire";
        let input = fs::read(&args[3]).expect("text records");
        let mut out = Vec::new();
        for message in records(&input) {
            let program = parse(std::str::from_utf8(message).expect("utf-8")).expect("parses");
            let encoded = if wire {
                wire_encode_program(&program)
            } else {
                fasl_encode_program(&program, &[0u8; 32])
            };
            out.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
            out.extend_from_slice(&encoded);
        }
        fs::write(&args[4], out).expect("records written");
        return ExitCode::SUCCESS;
    }

    let (form, path, mode) = (args[2].as_str(), &args[3], args[4].as_str());
    let bytes = fs::read(path).expect("records");
    let messages = records(&bytes);
    // Сесія створюється в усіх режимах, щоб її вартість віднімалася разом із базою.
    let mut session = Session::default();
    if mode == "base" {
        std::hint::black_box(&messages);
        return ExitCode::SUCCESS;
    }
    if mode == "decode" {
        for message in &messages {
            std::hint::black_box(load(form, message));
        }
        return ExitCode::SUCCESS;
    }

    let expected: Option<Vec<String>> = args.get(5).map(|file| {
        fs::read_to_string(file)
            .expect("expected")
            .lines()
            .map(str::to_owned)
            .collect()
    });
    for (index, message) in messages.iter().enumerate() {
        let program = load(form, message);
        match eval_lowered_expressions(&program, &mut session) {
            Ok(result) => {
                if let Some(expected) = &expected {
                    let got = result.value.to_string();
                    if got != expected[index] {
                        eprintln!("message {index}: expected {}, got {got}", expected[index]);
                        return ExitCode::from(3);
                    }
                }
                std::hint::black_box(result.value);
            }
            Err(error) => {
                eprintln!("message {index}: {error}");
                return ExitCode::from(4);
            }
        }
    }
    ExitCode::SUCCESS
}
