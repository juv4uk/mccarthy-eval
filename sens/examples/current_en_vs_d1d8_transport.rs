use sens::{
    eval_lowered_expressions, load_core_library, lower_program, pack_binary_source_tokens,
    packed_transport_accounting, parse, parse_binary_source_words, parse_canonical_binary, Expr,
    ExprKind, Session, Value,
};
use std::{env, fs, process};

const ENGLISH: &str = "english-surface";
const BINARY: &str = "canonical-d1d8";

fn parse_source(candidate: &str, source: &str) -> Result<Vec<Expr>, String> {
    match candidate {
        ENGLISH => parse(source).map_err(|error| format!("{error:?}")),
        BINARY => parse_canonical_binary(source).map_err(|error| format!("{error:?}")),
        other => Err(format!("unknown candidate: {other}")),
    }
}

fn trace(expr: &Expr) -> Result<String, String> {
    match &expr.kind {
        ExprKind::DomainIdentity(identity) => Ok(format!(
            "id:D{}:{:0width$b}",
            identity.width(),
            identity.packed_bits(),
            width = identity.width()
        )),
        ExprKind::DomainCall(identity, args) => {
            let mut out = format!(
                "call:D{}:{:0width$b}[",
                identity.width(),
                identity.packed_bits(),
                width = identity.width()
            );
            for (index, arg) in args.iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                out.push_str(&trace(arg)?);
            }
            out.push(']');
            Ok(out)
        }
        ExprKind::List(items) => {
            let mut out = String::from("list[");
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
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
            Err("legacy byte identity entered current D1-D8 transport preflight".to_owned())
        }
        other => Ok(format!("data:{other:?}")),
    }
}

fn trace_program(expressions: &[Expr]) -> Result<String, String> {
    expressions
        .iter()
        .map(trace)
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(";"))
}

fn node_count(expr: &Expr) -> usize {
    match &expr.kind {
        ExprKind::DomainCall(_, args) | ExprKind::List(args) => {
            1 + args.iter().map(node_count).sum::<usize>()
        }
        ExprKind::Pair(head, tail) => 1 + node_count(head) + node_count(tail),
        _ => 1,
    }
}

fn program_node_count(expressions: &[Expr]) -> usize {
    expressions.iter().map(node_count).sum()
}

fn prepared_session() -> Result<Session, String> {
    let mut session = Session::default();
    load_core_library(&mut session).map_err(|error| format!("{error:?}"))?;
    Ok(session)
}

fn value_fingerprint(value: &Value) -> String {
    if let Some(bit) = value.as_predicate_bit() {
        format!("D1:{}", u8::from(bit))
    } else {
        value.to_string()
    }
}

fn encode_hex(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for byte in text.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("write to String");
    }
    out
}

fn emit_optional<T: std::fmt::Display>(name: &str, value: Option<T>) {
    match value {
        Some(value) => println!("{name}={value}"),
        None => println!("{name}=NA"),
    }
}

fn run(candidate: &str, source: &str) -> Result<(), String> {
    let parsed = parse_source(candidate, source)?;
    let lowered = lower_program(&parsed);
    let semantic_trace = trace_program(&lowered)?;
    let lowered_nodes = program_node_count(&lowered);

    let mut session = prepared_session()?;
    let result =
        eval_lowered_expressions(&lowered, &mut session).map_err(|error| format!("{error:?}"))?;

    let mut semantic_payload_bits = None;
    let mut tail_unused_bits = None;
    let mut packed_bytes = None;
    let mut payload_container_bits = None;
    let mut packing_efficiency = None;

    if candidate == BINARY {
        let tokens = parse_binary_source_words(source).map_err(|error| format!("{error:?}"))?;
        let packed = pack_binary_source_tokens(&tokens);
        let accounting = packed_transport_accounting(&packed, 0);

        semantic_payload_bits = Some(accounting.semantic_payload_bits);
        tail_unused_bits = Some(accounting.tail_unused_bits);
        packed_bytes = Some(packed.byte_len());
        payload_container_bits = Some(packed.byte_len() * 8);
        packing_efficiency = accounting.utilization();
    }

    println!("TRACE_HEX={}", encode_hex(&semantic_trace));
    println!("VALUE_HEX={}", encode_hex(&value_fingerprint(&result.value)));
    println!("OUTPUT_HEX={}", encode_hex(&result.output.join("\n")));
    println!("SOURCE_BYTES={}", source.len());
    println!("LOWERED_AST_NODES={lowered_nodes}");
    emit_optional("SEMANTIC_PAYLOAD_BITS", semantic_payload_bits);
    println!("FRAMING_BITS=NA");
    emit_optional("TAIL_UNUSED_BITS", tail_unused_bits);
    println!("TOTAL_WIRE_BITS=NA");
    emit_optional("PACKED_BYTES", packed_bytes);
    emit_optional("PAYLOAD_CONTAINER_BITS", payload_container_bits);
    emit_optional("PACKING_EFFICIENCY", packing_efficiency);
    println!("CANONICAL_ARTIFACT_BYTES=NA");
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: {} <english-surface|canonical-d1d8> <source-file>",
            args.first()
                .map(String::as_str)
                .unwrap_or("current_en_vs_d1d8_transport")
        );
        process::exit(2);
    }

    let source = match fs::read_to_string(&args[2]) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("ERROR: read {}: {error}", args[2]);
            process::exit(2);
        }
    };

    if let Err(error) = run(&args[1], &source) {
        eprintln!("ERROR: {error}");
        process::exit(2);
    }
}
