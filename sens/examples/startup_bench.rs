//! Вартість старту: legacy MODE = session | bytes | decode | parse | macro | core.
//! #3629 додає лише діагностичні bare-root controls, не змінюючи старі modes:
//! root | root-macro | root-core.
//!
//! Важливо:
//! - Session::default() уже завантажує macro library;
//! - macro = default Session + повторне load_macro_library;
//! - core = default Session + load_core_library, який знову завантажує macros;
//! - root-* починаються з чистого Environment::root().
//!
//! Незалежні process modes не є вкладеними таймерами.
use sens::{
    fasl_decode_program, load_core_library, load_macro_library, parse, Environment, Session,
    CORE_LIBRARY_SOURCE,
};
use std::{env, fs, hint::black_box, process::ExitCode};

fn root_session() -> Session {
    Session {
        environment: Environment::root(),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(mode) = args.get(1).map(String::as_str) else {
        return ExitCode::from(2);
    };
    let fasl_path = args.get(2);

    let mut session = match mode {
        "root" | "root-macro" | "root-core" => root_session(),
        "session" | "bytes" | "decode" | "parse" | "macro" | "core" => Session::default(),
        _ => return ExitCode::from(2),
    };

    match mode {
        "root" | "session" => {}
        "bytes" => {
            black_box(fs::read(fasl_path.expect("bytes requires FASL path")).unwrap());
        }
        "decode" => {
            let bytes = fs::read(fasl_path.expect("decode requires FASL path")).unwrap();
            black_box(fasl_decode_program(&bytes).expect("fasl decodes"));
        }
        "parse" => {
            black_box(parse(CORE_LIBRARY_SOURCE).expect("core parses"));
        }
        "root-macro" | "macro" => {
            load_macro_library(&mut session).expect("macros load");
        }
        "root-core" | "core" => {
            load_core_library(&mut session).expect("core loads");
        }
        _ => unreachable!("mode validated above"),
    }

    black_box(&session);
    ExitCode::SUCCESS
}
