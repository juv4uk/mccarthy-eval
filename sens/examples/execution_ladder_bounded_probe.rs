use sens::{
    eval_lowered_expressions, load_core_library, lower_program, pack_binary_source_tokens,
    parse_binary_source_words, parse_canonical_binary, unpack_binary_source_words, Expr, ExprKind,
    Session, Value,
};
use std::{env, fs, process};

#[derive(Debug, Eq, PartialEq)]
struct SemanticSnapshot {
    trace: String,
    result_kind: &'static str,
    value: Option<String>,
    output: String,
    error_kind: Option<String>,
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
            Err("legacy byte identity entered bounded exact-domain probe".to_owned())
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

fn value_fingerprint(value: &Value) -> String {
    if let Some(bit) = value.as_predicate_bit() {
        format!("D1:{}", u8::from(bit))
    } else {
        value.to_string()
    }
}

fn evaluate(source: &str) -> Result<SemanticSnapshot, String> {
    let parsed = parse_canonical_binary(source).map_err(|error| format!("{error:?}"))?;
    let lowered = lower_program(&parsed);
    let semantic_trace = trace_program(&lowered)?;

    let mut session = Session::default();
    load_core_library(&mut session).map_err(|error| format!("core bootstrap: {error:?}"))?;

    match eval_lowered_expressions(&lowered, &mut session) {
        Ok(outcome) => Ok(SemanticSnapshot {
            trace: semantic_trace,
            result_kind: "VALUE",
            value: Some(value_fingerprint(&outcome.value)),
            output: outcome.output.join("\n"),
            error_kind: None,
        }),
        Err(error) => Ok(SemanticSnapshot {
            trace: semantic_trace,
            result_kind: "ERROR",
            value: None,
            output: String::new(),
            error_kind: Some(format!("{:?}", error.kind)),
        }),
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

fn bytes_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("write to String");
    }
    out
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        return Err(format!(
            "usage: {} <canonical-source-file>",
            args.first()
                .map(String::as_str)
                .unwrap_or("execution_ladder_bounded_probe")
        ));
    }

    let source = fs::read_to_string(&args[1])
        .map_err(|error| format!("read {}: {error}", args[1]))?;
    let source = source.trim();

    let tokens = parse_binary_source_words(source).map_err(|error| format!("{error:?}"))?;
    let widths = tokens
        .iter()
        .map(|token| token.word.width())
        .collect::<Vec<_>>();
    let original_words = tokens.iter().map(|token| token.word).collect::<Vec<_>>();

    let packed = pack_binary_source_tokens(&tokens);
    let decoded = unpack_binary_source_words(&packed, &widths)
        .ok_or_else(|| "P1 exact-width unpack failed".to_owned())?;
    if decoded != original_words {
        return Err("P1 exact-width unpack changed source words".to_owned());
    }

    let decoded_source = decoded
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");

    let original = evaluate(source)?;
    let round_tripped = evaluate(&decoded_source)?;
    if original != round_tripped {
        return Err(format!(
            "P1 semantic round-trip mismatch\noriginal={original:?}\ndecoded={round_tripped:?}"
        ));
    }

    println!("P1_ROUNDTRIP=1");
    println!("P1_BIT_LEN={}", packed.bit_len());
    println!(
        "P1_WIDTHS={}",
        widths
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("P1_PAYLOAD_HEX={}", bytes_hex(packed.bytes()));
    println!("TRACE_HEX={}", encode_hex(&original.trace));
    println!("RESULT_KIND={}", original.result_kind);
    println!(
        "VALUE_HEX={}",
        encode_hex(original.value.as_deref().unwrap_or(""))
    );
    println!("OUTPUT_HEX={}", encode_hex(&original.output));
    println!(
        "ERROR_KIND={}",
        original.error_kind.as_deref().unwrap_or("")
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR: {error}");
        process::exit(2);
    }
}
