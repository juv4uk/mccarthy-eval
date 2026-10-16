use sens::{
    eval_lowered_expressions, load_core_library, lower_program, parse, parse_canonical_binary, parse_mixed_exact_domain,
    Expr, ExprKind, Session, Value,
};
use std::{env, fs, process, time::Instant};

const ENGLISH: &str = "english-surface";
const UKRAINIAN: &str = "ukrainian-surface";
const BINARY: &str = "canonical-d1d8";

fn parse_source(candidate: &str, source: &str) -> Result<Vec<Expr>, String> {
    match candidate {
        ENGLISH => parse(source).map_err(|e| format!("{e:?}")),
        // Current source authority is the ratified Ukrainian surface. English
        // names remain display/compatibility projections and do not create
        // callable identity in this lane.
        UKRAINIAN => parse_mixed_exact_domain(source).map_err(|e| format!("{e:?}")),
        BINARY => parse_canonical_binary(source).map_err(|e| format!("{e:?}")),
        other => Err(format!("unknown candidate: {other}")),
    }
}

fn trace(expr: &Expr) -> Result<String, String> {
    match &expr.kind {
        ExprKind::DomainIdentity(id) => Ok(format!(
            "id:D{}:{:0width$b}",
            id.width(),
            id.packed_bits(),
            width = id.width()
        )),
        ExprKind::DomainCall(id, args) => {
            let mut out = format!(
                "call:D{}:{:0width$b}[",
                id.width(),
                id.packed_bits(),
                width = id.width()
            );
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&trace(arg)?);
            }
            out.push(']');
            Ok(out)
        }
        ExprKind::List(items) => {
            let mut out = "list[".to_owned();
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&trace(item)?);
            }
            out.push(']');
            Ok(out)
        }
        ExprKind::Pair(head, tail) => Ok(format!("pair[{},{}]", trace(head)?, trace(tail)?)),
        ExprKind::Local { depth, index } => Ok(format!("local:{depth}:{index}")),
        ExprKind::Sid(_) | ExprKind::Call(_, _) => {
            Err("legacy byte identity entered current D1-D8 CPU preflight".to_owned())
        }
        other => Ok(format!("data:{other:?}")),
    }
}

fn trace_program(expressions: &[Expr]) -> Result<String, String> {
    let mut parts = Vec::with_capacity(expressions.len());
    for expression in expressions {
        parts.push(trace(expression)?);
    }
    Ok(parts.join(";"))
}

fn prepared_session() -> Result<Session, String> {
    let mut session = Session::default();
    load_core_library(&mut session).map_err(|e| format!("{e:?}"))?;
    Ok(session)
}

fn encode_hex(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for byte in text.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("write to String");
    }
    out
}

fn value_fingerprint(value: &Value) -> String {
    if let Some(bit) = value.as_predicate_bit() {
        format!("D1:{}", u8::from(bit))
    } else {
        value.to_string()
    }
}

fn emit(elapsed_ns: u128, trace: &str, value: &str, output: &str) {
    println!("ELAPSED_NS={elapsed_ns}");
    println!("TRACE_HEX={}", encode_hex(trace));
    println!("VALUE_HEX={}", encode_hex(value));
    println!("OUTPUT_HEX={}", encode_hex(output));
}

fn preflight(candidate: &str, source: &str) -> Result<(), String> {
    let parsed = parse_source(candidate, source)?;
    let lowered = lower_program(&parsed);
    let semantic_trace = trace_program(&lowered)?;
    // This preflight pairs ratified Ukrainian D3 source heads with canonical
    // exact-width binary source (QUOTE/CAR only). It deliberately does not
    // grant callable meaning to English aliases or use a legacy SID table.
    let mut session = Session::default();
    let result = eval_lowered_expressions(&lowered, &mut session).map_err(|e| format!("{e:?}"))?;
    emit(
        0,
        &semantic_trace,
        &value_fingerprint(&result.value),
        &result.output.join("\n"),
    );
    Ok(())
}

fn measure(candidate: &str, phase: &str, source: &str, repeat: usize) -> Result<(), String> {
    match phase {
        "baseline" => {
            std::hint::black_box(source.len());
            emit(0, "", "", "");
        }
        "session" => {
            let started = Instant::now();
            let _session = prepared_session()?;
            emit(started.elapsed().as_nanos(), "", "", "");
        }
        "ingest" => {
            let started = Instant::now();
            let _parsed = parse_source(candidate, source)?;
            emit(started.elapsed().as_nanos(), "", "", "");
        }
        "lower" => {
            let parsed = parse_source(candidate, source)?;
            let started = Instant::now();
            let _lowered = lower_program(&parsed);
            emit(started.elapsed().as_nanos(), "", "", "");
        }
        "ready" => {
            let parsed = parse_source(candidate, source)?;
            let lowered = lower_program(&parsed);
            let session = prepared_session()?;
            std::hint::black_box(&lowered);
            std::hint::black_box(&session);
            emit(0, "", "", "");
        }
        "execute" => {
            let parsed = parse_source(candidate, source)?;
            let lowered = lower_program(&parsed);
            let mut session = prepared_session()?;
            let started = Instant::now();
            let result =
                eval_lowered_expressions(&lowered, &mut session).map_err(|e| format!("{e:?}"))?;
            emit(
                started.elapsed().as_nanos(),
                "",
                &value_fingerprint(&result.value),
                &result.output.join("\n"),
            );
        }
        "full" => {
            let started = Instant::now();
            let mut session = prepared_session()?;
            let parsed = parse_source(candidate, source)?;
            let lowered = lower_program(&parsed);
            let result =
                eval_lowered_expressions(&lowered, &mut session).map_err(|e| format!("{e:?}"))?;
            emit(
                started.elapsed().as_nanos(),
                "",
                &value_fingerprint(&result.value),
                &result.output.join("\n"),
            );
        }
        "repeated" => {
            if repeat == 0 {
                return Err("repeat must be greater than zero".to_owned());
            }
            let parsed = parse_source(candidate, source)?;
            let lowered = lower_program(&parsed);
            let mut session = prepared_session()?;
            let started = Instant::now();
            let mut last = None;
            for _ in 0..repeat {
                last = Some(
                    eval_lowered_expressions(&lowered, &mut session)
                        .map_err(|e| format!("{e:?}"))?,
                );
            }
            let result = last.expect("repeat > 0");
            emit(
                started.elapsed().as_nanos(),
                "",
                &value_fingerprint(&result.value),
                &result.output.join("\n"),
            );
        }
        other => return Err(format!("unknown phase: {other}")),
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args.len() > 5 {
        return Err(format!(
            "usage: {} <candidate> <preflight|baseline|session|ingest|lower|ready|execute|full|repeated> <source-file> [repeat]",
            args.first().map(String::as_str).unwrap_or("current_en_vs_d1d8_cpu")
        ));
    }
    let candidate = &args[1];
    let phase = &args[2];
    let source = fs::read_to_string(&args[3]).map_err(|e| format!("read {}: {e}", args[3]))?;
    let repeat = if args.len() == 5 {
        args[4]
            .parse::<usize>()
            .map_err(|e| format!("invalid repeat count: {e}"))?
    } else {
        1
    };

    if phase == "preflight" {
        preflight(candidate, &source)
    } else {
        measure(candidate, phase, &source, repeat)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR: {error}");
        process::exit(2);
    }
}
