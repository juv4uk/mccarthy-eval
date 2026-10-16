//! Bootstrap registry for first-class `Value::Builtin` mechanisms. Contract
//! 2.1 introduced ordinary callable builtin values; Contract 6.0 narrows the
//! lookup rule for Canon 0+7 only. Non-Canon builtins remain ordinary lexical
//! bindings. Historical Canon bootstrap entries may still exist in the root
//! environment as implementation detail, but they are never semantic authority:
//! the immutable Canon resolver wins before `Environment` lookup for every
//! reserved Canon spelling.
//!
//! ADR-007 extends the meaning-first shape beyond Canon. Ordinary public
//! operations stay shadowable lexical values, but peer human spellings may be
//! installed as direct names of one `Value::Builtin` allocation instead of as
//! aliases through another human surface. ADD plus the stable arithmetic and
//! comparison operator peers are now proved runtime slices.
//!
//! Batch 1 (2026-08-23): car cdr cons eq atom + - * / < > =.
//! Batch 2 (2026-09-07): eager string/symbol, Unicode-codepoint, and digest
//! mechanisms are ordinary builtin values too; the evaluator does not need
//! head-name dispatch for them.
//! Batch 3 (2026-09-07): JSON decode is likewise an eager first-class builtin.
//! Batch 4 (2026-09-07): print/read/eval reflection mechanisms are ordinary
//! first-class builtins; only genuine syntax remains in evaluator dispatch.

use crate::environment::Environment;
use crate::eval::arithmetic::{
    exact_value,
};
use crate::eval::special_forms::json::json_parse_values;
use crate::eval::special_forms::{
    codepoint_to_string_values,
    princ_values, print_values, read_all_values, read_values, sha256_hex_values,
    string_append_values, string_first_values, string_predicate_values,
    string_rest_values, string_to_codepoint_values, string_to_symbol_values,
    symbol_to_string_values, write_to_string_values,
};
use crate::{Exactness, NumericBuffer, Rational, Span, Value};




fn ntp_query_raw_value(
    host: &str,
    timeout_ms: u64,
    span: Span,
) -> Result<Value, crate::LanguageError> {
    use std::net::{ToSocketAddrs, UdpSocket};
    use std::time::Duration;
    let timeout_ms = timeout_ms.min(5_000);
    let address = (host, 123)
        .to_socket_addrs()
        .map_err(|_| {
            crate::LanguageError::new(
                crate::ErrorKind::Type,
                "ntp-query-raw cannot resolve host",
                span,
            )
        })?
        .next()
        .ok_or_else(|| {
            crate::LanguageError::new(
                crate::ErrorKind::Type,
                "ntp-query-raw host has no address",
                span,
            )
        })?;
    let socket = UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
            socket.set_write_timeout(Some(Duration::from_millis(timeout_ms)))?;
            socket.connect(address)?;
            Ok(socket)
        })
        .map_err(|_| {
            crate::LanguageError::new(
                crate::ErrorKind::Type,
                "ntp-query-raw socket unavailable",
                span,
            )
        })?;
    let mut request = [0u8; 48];
    request[0] = 0x23;
    if socket.send(&request).is_err() {
        return Ok(Value::list([
            Value::Symbol(std::rc::Rc::from("rejected")),
            Value::Symbol(std::rc::Rc::from("send-failed")),
        ]));
    }
    let mut response = [0u8; 512];
    let size = match socket.recv(&mut response) {
        Ok(size) => size,
        Err(_) => {
            return Ok(Value::list([
                Value::Symbol(std::rc::Rc::from("rejected")),
                Value::Symbol(std::rc::Rc::from("receive-failed")),
            ]));
        }
    };
    if size < 48 {
        return Ok(Value::list([
            Value::Symbol(std::rc::Rc::from("rejected")),
            Value::Symbol(std::rc::Rc::from("short-response")),
        ]));
    }

    let mode = response[0] & 0x07;
    let stratum = response[1];
    let seconds = u32::from_be_bytes([response[40], response[41], response[42], response[43]]);
    let fraction = u32::from_be_bytes([response[44], response[45], response[46], response[47]]);

    Ok(Value::list([
        Value::Symbol(std::rc::Rc::from("ntp-fields")),
        Value::String(std::rc::Rc::from(host)),
        exact_value(Rational::integer(i64::from(mode))),
        exact_value(Rational::integer(i64::from(stratum))),
        exact_value(Rational::integer(i64::from(seconds))),
        exact_value(Rational::integer(i64::from(fraction))),
    ]))
}

/// Арність визначається конкретним механізмом, а не історичним W8/SID.
/// Точний домен виклику перевіряє dispatch до входу в цей backend.
fn exact_args(
    args: &[Value],
    expected: usize,
    span: Span,
) -> Result<(), crate::LanguageError> {
    if args.len() != expected {
        return Err(crate::LanguageError::new(
            crate::ErrorKind::Arity,
            format!(
                "невірна кількість аргументів: очікується {expected}, отримано {}",
                args.len()
            ),
            span,
        ));
    }
    Ok(())
}

// Примітиви мови за кодом СЕНС (власник, 2026-09-26: Rust знає лише коди).
/// make-vector
pub(super) fn prim_01010000(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 1, span)?;
        match &args[0] {
            Value::Number(f, Exactness::Exact) if *f >= 0.0 && f.fract() == 0.0 =>
                Ok(Value::vector(std::iter::repeat_n(Value::Nil, *f as usize))),
            _ => Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "make-vector expects an exact non-negative integer · make-vector ochikuie tochnyi nenulevyi tsilyi · make-vector erwartet eine exakte nichtnegative ganze Zahl", span)),
        }
    }

/// vector
pub(super) fn prim_01001111(args: &[Value], _env: &Environment, _span: Span) -> Result<Value, crate::LanguageError> {
        Ok(Value::vector(args.iter().cloned()))
    }

/// mono-ns
pub(super) fn prim_01011010(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 0, span)?;
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        let elapsed = START.get_or_init(std::time::Instant::now).elapsed();
        Ok(exact_value(Rational::integer(elapsed.as_nanos() as i64)))
    }

/// unix-time-now
pub(super) fn prim_01011011(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 0, span)?;
        let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| crate::LanguageError::new(crate::ErrorKind::Type,
                "unix-time-now is unavailable before the Unix epoch", span))?;
        let seconds = i64::try_from(duration.as_secs()).map_err(|_| {
            crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                "unix-time-now seconds exceed the signed 64-bit range", span)
        })?;
        Ok(Value::list([
            Value::Symbol(std::rc::Rc::from("unix-time")),
            exact_value(Rational::integer(seconds)),
            exact_value(Rational::integer(duration.subsec_nanos() as i64)),
        ]))
    }

/// ntp-query-raw
pub(super) fn prim_01011100(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 2, span)?;
        let host = match &args[0] {
            Value::String(value) => value.as_ref(),
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "ntp-query-raw expects host string", span)),
        };
        let timeout = match &args[1] {
            Value::Number(value, Exactness::Exact) if *value >= 0.0 && value.fract() == 0.0 => *value as u64,
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "ntp-query-raw expects exact timeout milliseconds", span)),
        };
        ntp_query_raw_value(host, timeout, span)
    }

/// timezone-declarations-raw
pub(super) fn prim_01011101(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 0, span)?;
        let tz_value = std::env::var("TZ").ok().filter(|value| !value.is_empty())
            .map(|value| Value::String(std::rc::Rc::from(value))).unwrap_or(Value::Nil);
        let etc_timezone_value = std::fs::read_to_string("/etc/timezone").ok()
            .map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
            .map(|value| Value::String(std::rc::Rc::from(value))).unwrap_or(Value::Nil);
        Ok(Value::list([
            Value::Symbol(std::rc::Rc::from("timezone-declarations")), tz_value, etc_timezone_value,
        ]))
    }

/// vector-length
pub(super) fn prim_01010001(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 1, span)?;
        match &args[0] {
            Value::Vector(vec) => Ok(Value::Number(vec.borrow().len() as f64, Exactness::Exact)),
            _ => Err(crate::LanguageError::new(crate::ErrorKind::Type, "vector-length expects a vector", span)),
        }
    }

/// vector-ref
pub(super) fn prim_01010010(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 2, span)?;
        let index = match &args[1] {
            Value::Number(f, Exactness::Exact) if *f >= 0.0 && f.fract() == 0.0 && *f <= usize::MAX as f64 => *f as usize,
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "vector-ref expects an exact non-negative integer index", span)),
        };
        match &args[0] {
            Value::Vector(vec) => vec.borrow().get(index).cloned().ok_or_else(|| {
                crate::LanguageError::new(crate::ErrorKind::InvalidForm,
                    format!("vector-ref index {index} out of bounds for length {}", vec.borrow().len()), span)
            }),
            _ => Err(crate::LanguageError::new(crate::ErrorKind::Type, "vector-ref expects a vector", span)),
        }
    }

/// vector-set!
pub(super) fn prim_01010011(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 3, span)?;
        let index = match &args[1] {
            Value::Number(f, Exactness::Exact) if *f >= 0.0 && f.fract() == 0.0 && *f <= usize::MAX as f64 => *f as usize,
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "vector-set! expects an exact non-negative integer index", span)),
        };
        match &args[0] {
            Value::Vector(vec) => {
                let mut vec = vec.borrow_mut();
                if index >= vec.len() {
                    let len = vec.len();
                    return Err(crate::LanguageError::new(crate::ErrorKind::InvalidForm,
                        format!("vector-set! index {index} out of bounds for length {len}"), span));
                }
                vec[index] = args[2].clone();
                Ok(Value::Nil)
            }
            _ => Err(crate::LanguageError::new(crate::ErrorKind::Type, "vector-set! expects a vector", span)),
        }
    }

/// i32-buffer
pub(super) fn prim_01010100(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        let mut values = Vec::with_capacity(args.len());
        for value in args {
            let integer = match value {
                Value::Number(number, Exactness::Exact) if number.fract() == 0.0 => *number as i64,
                Value::Rational(rational) if rational.is_integer() => rational.as_precise_i64().ok_or_else(|| {
                    crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                        "i32-buffer element is outside the signed 32-bit range", span)
                })?,
                _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                    "i32-buffer expects exact integer elements", span)),
            };
            values.push(i32::try_from(integer).map_err(|_| {
                crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                    "i32-buffer element is outside the signed 32-bit range", span)
            })?);
        }
        Ok(Value::NumericBuffer(NumericBuffer::I32(values.into())))
    }

/// f32-buffer
pub(super) fn prim_01010101(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        let mut values = Vec::with_capacity(args.len());
        for value in args {
            let number = match value {
                Value::Number(number, _) => *number,
                Value::Rational(rational) => rational.as_f64(),
                _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                    "f32-buffer expects numeric elements", span)),
            };
            let narrowed = number as f32;
            if !number.is_finite() || !narrowed.is_finite() {
                return Err(crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                    "f32-buffer element is outside the finite binary32 domain", span));
            }
            values.push(narrowed);
        }
        Ok(Value::NumericBuffer(NumericBuffer::F32(values.into())))
    }

/// string-slice
pub(super) fn prim_01000001(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        super::special_forms::evaluate_string_slice(args, span)
    }

/// string-append
pub(super) fn prim_00111010(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        string_append_values(args, span)
    }

/// string?
pub(super) fn prim_00100100(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        string_predicate_values(args, span)
    }

/// symbol->string
pub(super) fn prim_01000010(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        symbol_to_string_values(args, span)
    }

/// string->symbol
pub(super) fn prim_01000011(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        string_to_symbol_values(args, span)
    }

/// string-first
pub(super) fn prim_00111111(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        string_first_values(args, span)
    }

/// string-rest
pub(super) fn prim_01000000(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        string_rest_values(args, span)
    }

/// codepoint->string
pub(super) fn prim_01000100(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        codepoint_to_string_values(args, span)
    }

/// string->codepoint
pub(super) fn prim_01000101(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        string_to_codepoint_values(args, span)
    }

/// sha256-hex
pub(super) fn prim_10100001(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        sha256_hex_values(args, span)
    }

/// json-parse
pub(super) fn prim_10100000(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        json_parse_values(args, span)
    }

/// print
pub(super) fn prim_01001000(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        print_values(args, env, span)
    }

/// princ
pub(super) fn prim_01001001(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        princ_values(args, env, span)
    }

/// write-to-string
pub(super) fn prim_01001100(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        write_to_string_values(args, env, span)
    }

/// read
pub(super) fn prim_01001010(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        read_values(args, env, span)
    }

/// read-all
pub(super) fn prim_01001011(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        read_all_values(args, env, span)
    }

/// numeric-buffer?
pub(super) fn prim_00100110(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 1, span)?;
        Ok(Value::predicate_bit(matches!(args[0], Value::NumericBuffer(_))))
    }

/// numeric-buffer-type
pub(super) fn prim_01010110(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 1, span)?;
        let name = match &args[0] {
            Value::NumericBuffer(NumericBuffer::I32(_)) => "i32",
            Value::NumericBuffer(NumericBuffer::F32(_)) => "f32",
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "numeric-buffer-type expects a numeric buffer", span)),
        };
        Ok(Value::Symbol(name.into()))
    }

/// numeric-buffer-length
pub(super) fn prim_01010111(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 1, span)?;
        let length = match &args[0] {
            Value::NumericBuffer(NumericBuffer::I32(values)) => values.len(),
            Value::NumericBuffer(NumericBuffer::F32(values)) => values.len(),
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "numeric-buffer-length expects a numeric buffer", span)),
        };
        Ok(Value::Number(length as f64, Exactness::Exact))
    }

/// numeric-buffer-ref
pub(super) fn prim_01011000(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 2, span)?;
        let index = match args[1] {
            Value::Number(number, Exactness::Exact)
                if number >= 0.0 && number.fract() == 0.0 && number <= usize::MAX as f64 => number as usize,
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "numeric-buffer-ref expects an exact non-negative integer index", span)),
        };
        match &args[0] {
            Value::NumericBuffer(NumericBuffer::I32(values)) => values.get(index)
                .map(|value| Value::Number(f64::from(*value), Exactness::Exact)),
            Value::NumericBuffer(NumericBuffer::F32(values)) => values.get(index)
                .map(|value| Value::Number(f64::from(*value), Exactness::Inexact)),
            _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "numeric-buffer-ref expects a numeric buffer", span)),
        }.ok_or_else(|| crate::LanguageError::new(crate::ErrorKind::InvalidForm,
            "numeric-buffer-ref index is out of bounds", span))
    }

/// numeric-buffer-map
pub(super) fn prim_01011001(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 2, span)?;
        match &args[1] {
            Value::NumericBuffer(NumericBuffer::I32(input)) => {
                let mut output = Vec::with_capacity(input.len());
                for element in input.iter() {
                    let result = super::invoke_value(&args[0],
                        &[Value::Number(f64::from(*element), Exactness::Exact)], env, span)?;
                    let integer = match &result {
                        Value::Number(number, Exactness::Exact) if number.fract() == 0.0 => *number as i64,
                        Value::Rational(rational) if rational.is_integer() => rational.as_precise_i64().ok_or_else(|| {
                            crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                                "numeric-buffer-map i32 result is outside the signed 32-bit range", span)
                        })?,
                        _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                            "numeric-buffer-map over i32 requires exact integer results", span)),
                    };
                    output.push(i32::try_from(integer).map_err(|_| {
                        crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                            "numeric-buffer-map i32 result is outside the signed 32-bit range", span)
                    })?);
                }
                Ok(Value::NumericBuffer(NumericBuffer::I32(output.into())))
            }
            Value::NumericBuffer(NumericBuffer::F32(input)) => {
                let mut output = Vec::with_capacity(input.len());
                for bits in input.iter() {
                    let result = super::invoke_value(&args[0],
                        &[Value::Number(f64::from(*bits), Exactness::Inexact)], env, span)?;
                    let number = match &result {
                        Value::Number(number, _) => *number,
                        Value::Rational(rational) => rational.as_f64(),
                        _ => return Err(crate::LanguageError::new(crate::ErrorKind::Type,
                            "numeric-buffer-map over f32 requires numeric results", span)),
                    };
                    let narrowed = number as f32;
                    if !number.is_finite() || !narrowed.is_finite() {
                        return Err(crate::LanguageError::new(crate::ErrorKind::NumericOverflow,
                            "numeric-buffer-map f32 result is outside the finite binary32 domain", span));
                    }
                    output.push(narrowed);
                }
                Ok(Value::NumericBuffer(NumericBuffer::F32(output.into())))
            }
            _ => Err(crate::LanguageError::new(crate::ErrorKind::Type,
                "numeric-buffer-map expects a numeric buffer", span)),
        }
    }

/// env
pub(super) fn prim_01001110(args: &[Value], env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
        exact_args(args, 0, span)?;
        let mut items = Vec::new();
        for (name, value) in env.snapshot() {
            items.push(Value::Pair(std::rc::Rc::new(Value::String(name)), std::rc::Rc::new(value)));
        }
        let mut list = Value::Nil;
        for item in items.into_iter().rev() {
            list = Value::Pair(std::rc::Rc::new(item), std::rc::Rc::new(list));
        }
        Ok(list)
    }

// --- Рядкові примітиви за кодом (власник, 2026-09-26: «швидкі рядкові функції
// в ядрі»). Раніше — визначення мовою через string-first/string-rest, які
// щоразу копіюють решту рядка (квадратично на довгому тексті). Поведінка,
// включно з відповідями й помилками на не-рядках, повторює мовні версії.

fn string_rest_or_error(value: &Value, span: Span) -> Result<Value, crate::LanguageError> {
    string_rest_values(std::slice::from_ref(value), span)
}

fn string_first_or_error(value: &Value, span: Span) -> Result<Value, crate::LanguageError> {
    string_first_values(std::slice::from_ref(value), span)
}

/// string-empty? — це `eq?` з порожнім рядком (та сама відповідь, що й раніше).
fn string_empty_answer(value: &Value, span: Span) -> Result<Value, crate::LanguageError> {
    super::special_forms::eq_values(value.clone(), Value::String(std::rc::Rc::from("")), span)
}

fn is_empty_string(value: &Value) -> bool {
    matches!(value, Value::String(text) if text.is_empty())
}

/// string-empty?
pub(super) fn prim_00111100(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
    exact_args(args, 1, span)?;
    string_empty_answer(&args[0], span)
}

/// string-length — кількість символів (кодових точок).
pub(super) fn prim_00111011(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
    exact_args(args, 1, span)?;
    match &args[0] {
        Value::String(text) => Ok(exact_value(Rational::integer(text.chars().count() as i64))),
        other => {
            // Мовна версія: string-empty? → (0), далі string-rest → помилка Type.
            string_empty_answer(other, span)?;
            string_rest_or_error(other, span)?;
            unreachable!("string-rest accepts only strings")
        }
    }
}

fn string_prefix_answer(prefix: &Value, text: &Value, span: Span) -> Result<Value, crate::LanguageError> {
    match (prefix, text) {
        (Value::String(p), Value::String(t)) => Ok(Value::predicate_bit(t.starts_with(p.as_ref()))),
        _ => {
            // Той самий порядок кроків, що в мовній версії.
            if is_empty_string(prefix) {
                return Ok(Value::predicate_bit(true));
            }
            string_empty_answer(prefix, span)?;
            if is_empty_string(text) {
                return Ok(Value::predicate_bit(false));
            }
            string_empty_answer(text, span)?;
            string_first_or_error(prefix, span)?;
            string_first_or_error(text, span)?;
            unreachable!("string-first accepts only strings")
        }
    }
}

/// string-prefix? prefix s
pub(super) fn prim_00111101(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
    exact_args(args, 2, span)?;
    string_prefix_answer(&args[0], &args[1], span)
}

/// string-contains? needle s
pub(super) fn prim_00111110(args: &[Value], _env: &Environment, span: Span) -> Result<Value, crate::LanguageError> {
    exact_args(args, 2, span)?;
    match (&args[0], &args[1]) {
        (Value::String(needle), Value::String(text)) => {
            Ok(Value::predicate_bit(text.contains(needle.as_ref())))
        }
        (needle, text) => {
            // Мовна версія: спершу string-prefix?, потім string-empty?, потім string-rest.
            if string_prefix_answer(needle, text, span)? == Value::predicate_bit(true) {
                return Ok(Value::predicate_bit(true));
            }
            if is_empty_string(text) {
                return Ok(Value::predicate_bit(false));
            }
            string_empty_answer(text, span)?;
            string_rest_or_error(text, span)?;
            unreachable!("string-rest accepts only strings")
        }
    }
}

#[cfg(test)]
mod arity_without_legacy_id_tests {
    use super::*;

    #[test]
    fn mechanism_arity_keeps_kind_and_source_span_without_w8_identity() {
        let span = Span { start: 12, end: 34 };
        for (expected, supplied) in [(0, 1), (1, 0), (1, 2), (2, 1), (3, 4)] {
            let args = vec![Value::Nil; supplied];
            let error = exact_args(&args, expected, span)
                .expect_err("невідповідність кількості аргументів має завершитися помилкою");
            assert_eq!(error.kind, crate::ErrorKind::Arity);
            assert_eq!(error.span, span);
            assert!(error.message.contains(&format!("очікується {expected}")));
            assert!(error.message.contains(&format!("отримано {supplied}")));
        }
        for expected in 0..=3 {
            let args = vec![Value::Nil; expected];
            exact_args(&args, expected, span)
                .expect("точна кількість аргументів приймається");
        }
    }

    #[test]
    fn live_predicate_backend_keeps_d1_and_rejects_wrong_arity() {
        let env = Environment::root();
        let span = Span { start: 4, end: 11 };
        let value = prim_00100110(&[Value::Nil], &env, span)
            .expect("механізм numeric-buffer? виконується без W8 у перевірці арності");
        assert_eq!(value.as_predicate_bit(), Some(false));

        let error = prim_00100110(&[], &env, span)
            .expect_err("нуль аргументів не є допустимою арністю");
        assert_eq!(error.kind, crate::ErrorKind::Arity);
        assert_eq!(error.span, span);

        let too_many = prim_00100110(&[Value::Nil, Value::Nil], &env, span)
            .expect_err("надлишковий аргумент не є допустимим");
        assert_eq!(too_many.kind, crate::ErrorKind::Arity);
    }

    #[test]
    fn string_predicate_backend_keeps_exact_d1_result_without_sid8() {
        let env = Environment::root();
        let span = Span::default();
        let value = Value::String(std::rc::Rc::from(""));
        let result = prim_00111100(&[value], &env, span)
            .expect("порожній рядок обробляється поточним предикатним механізмом");
        assert_eq!(result.as_predicate_bit(), Some(true));
        let error = prim_00111100(&[], &env, span)
            .expect_err("помилка арності лишається явною");
        assert_eq!(error.kind, crate::ErrorKind::Arity);
    }
}
