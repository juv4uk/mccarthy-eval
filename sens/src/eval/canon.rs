//! Evaluator mechanism routing.
//!
//! Contract 11 canonical identity is exact domain + exact bits. Historical
//! CoreDomainIdentity tables remain a compatibility/backend lane while migrated D3 calls
//! route directly from `CoreDomainIdentity` without reconstructing a byte.

use super::{
    arithmetic, builtins, closures, necessary_forms,
    profile_mechanisms_generated::{profile_mechanism_route, ProfileMechanismRouteKind},
    special_forms,
};
use crate::{semantic_registry, Environment, ErrorKind, LanguageError, CoreDomainIdentity, Span, Value};
use crate::CoreDomainIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SidRouteKind {
    ValueCall,
    SpecialForm,
}

/// Сумісний transport не містить власної таблиці семантичних ролей.
/// Координата береться лише з чинної ратифікованої драбини D3.
pub(crate) fn route_kind_for_sid(sid: CoreDomainIdentity) -> Option<SidRouteKind> {
    let CoreDomainIdentity::D3(word) =
        semantic_registry::compatibility_d3_route_from_ratified_domains(sid)?
    else {
        return None;
    };
    match word.word().packed_bits() {
        0b001 | 0b110 => Some(SidRouteKind::SpecialForm),
        0b010 | 0b011 | 0b100 | 0b101 | 0b111 => Some(SidRouteKind::ValueCall),
        _ => None,
    }
}

/// Optional source/UI routing only. The returned value is the function SID;
/// no named meaning is materialized.
pub(crate) fn routed_sid_for_surface(surface: &str) -> Option<CoreDomainIdentity> {
    let sid = semantic_registry::semantic_id_for_surface(surface)?;
    route_kind_for_sid(sid)?;
    Some(sid)
}

/// Admit immutable source surfaces from the ratified exact-domain route first.
/// D5 definitions without admitted mechanisms remain bindable by the language;
/// mere residency is never sufficient to freeze a binding or grant callability.
fn immutable_exact_surface_identity(surface: &str) -> Option<CoreDomainIdentity> {
    let identity = super::lower::immutable_surface_domain_identity(surface)?;
    (matches!(identity, CoreDomainIdentity::D3(_))
        || domain_primitive_kind(identity).is_some()
        || necessary_forms::identity_for_domain_identity(identity).is_some())
    .then_some(identity)
}

pub(crate) fn is_reserved_surface(surface: &str) -> bool {
    if immutable_exact_surface_identity(surface).is_some() {
        return true;
    }
    routed_sid_for_surface(surface).is_some()
}

/// Surface, яку не можна перевизначити: Canon, necessary form, або примітив.
/// Exact domain + exact bits own the canonical diagnostic. The old eight-bit
/// compatibility route is only consulted for unmigrated source surfaces.
pub(crate) fn ensure_bindable(surface: &str, span: Span) -> Result<(), LanguageError> {
    if let Some(identity) = immutable_exact_surface_identity(surface) {
        return Err(immutable_domain_binding_error(identity.into(), span));
    }
    if let Some(sid) = routed_sid_for_surface(surface) {
        return Err(immutable_surface_error(surface, sid, span));
    }
    if let Some(sid) = semantic_registry::admitted_semantic_id_for_surface(surface) {
        if has_primitive(sid) || necessary_forms::identity_for_semantic_id(sid).is_some() {
            return Err(immutable_surface_error(surface, sid, span));
        }
    }
    Ok(())
}

fn immutable_surface_error(surface: &str, sid: CoreDomainIdentity, span: Span) -> LanguageError {
    LanguageError::new(
        ErrorKind::InvalidForm,
        format!(
            "surface routes to immutable function SID · surface маршрутизується до незмінного function SID · Surface verweist auf unveränderliche Funktions-SID: {surface} -> {sid}"
        ),
        span,
    )
}

pub(crate) fn ensure_bindable_sid(sid: CoreDomainIdentity, span: Span) -> Result<(), LanguageError> {
    Err(LanguageError::new(
        ErrorKind::InvalidForm,
        format!(
            "surface routes to immutable function SID · surface маршрутизується до незмінного function SID · Surface verweist auf unveränderliche Funktions-SID: {sid} -> {sid}"
        ),
        span,
    ))
}

pub(crate) fn immutable_domain_binding_error(
    identity: crate::DomainIdentity,
    span: Span,
) -> LanguageError {
    LanguageError::new(
        ErrorKind::InvalidForm,
        format!(
            "canonical name is immutable after exact-domain lowering · канонічне ім'я незмінне після exact-domain lowering · kanonischer Name ist nach Exact-Domain-Lowering unveränderlich: D{} {}",
            identity.width(),
            identity
        ),
        span,
    )
}

fn exact_args(
    sid: crate::CoreDomainIdentity,
    args: &[Value],
    expected: usize,
    span: Span,
) -> Result<(), LanguageError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(LanguageError::new(
        ErrorKind::Arity,
        format!(
            "{sid}: expected / ochikuvalosia / erwartet {expected}; received / otrymano / erhalten {}",
            args.len()
        ),
        span,
    ))
}

type PrimitiveFn = fn(&[Value], &Environment, Span) -> Result<Value, LanguageError>;

/// Вузька таблиця тільки історичних backend-механізмів.
///
/// Цей сумісний транспорт ніколи не визначає координату, роль чи закон
/// поточних D1–D10. Виклики через CoreDomainIdentity мають власні
/// допущені механізми і перевіряються ПЕРЕД цим маршрутом.
/// Замість плоского масиву 256 слотів дозволяємо лише явні старі
/// механізми, які ще мають підтверджених споживачів.
fn legacy_backend_primitive(sid: CoreDomainIdentity) -> Option<PrimitiveFn> {
    match sid.packed_byte() {
        0b00001100 => Some(prim_00001100),
        0b00001101 => Some(prim_00001101),
        0b00001110 => Some(prim_00001110),
        0b00001111 => Some(prim_00001111),
        0b00011010 => Some(prim_00011010),
        0b00011011 => Some(prim_00011011),
        0b00011100 => Some(prim_00011100),
        0b01001101 => Some(prim_01001101),
        0b00111011 => Some(builtins::prim_00111011),
        0b00111100 => Some(builtins::prim_00111100),
        0b00111101 => Some(builtins::prim_00111101),
        0b00111110 => Some(builtins::prim_00111110),
        0b01010000 => Some(builtins::prim_01010000),
        0b01001111 => Some(builtins::prim_01001111),
        0b01011010 => Some(builtins::prim_01011010),
        0b01011011 => Some(builtins::prim_01011011),
        0b01011100 => Some(builtins::prim_01011100),
        0b01011101 => Some(builtins::prim_01011101),
        0b01010001 => Some(builtins::prim_01010001),
        0b01010010 => Some(builtins::prim_01010010),
        0b01010011 => Some(builtins::prim_01010011),
        0b01010100 => Some(builtins::prim_01010100),
        0b01010101 => Some(builtins::prim_01010101),
        0b01000001 => Some(builtins::prim_01000001),
        0b00111010 => Some(builtins::prim_00111010),
        0b00100100 => Some(builtins::prim_00100100),
        0b01000010 => Some(builtins::prim_01000010),
        0b01000011 => Some(builtins::prim_01000011),
        0b00111111 => Some(builtins::prim_00111111),
        0b01000000 => Some(builtins::prim_01000000),
        0b01000100 => Some(builtins::prim_01000100),
        0b01000101 => Some(builtins::prim_01000101),
        0b10100001 => Some(builtins::prim_10100001),
        0b10100000 => Some(builtins::prim_10100000),
        0b01001000 => Some(builtins::prim_01001000),
        0b01001001 => Some(builtins::prim_01001001),
        0b01001100 => Some(builtins::prim_01001100),
        0b01001010 => Some(builtins::prim_01001010),
        0b01001011 => Some(builtins::prim_01001011),
        0b00100110 => Some(builtins::prim_00100110),
        0b01010110 => Some(builtins::prim_01010110),
        0b01010111 => Some(builtins::prim_01010111),
        0b01011000 => Some(builtins::prim_01011000),
        0b01011001 => Some(builtins::prim_01011001),
        0b01001110 => Some(builtins::prim_01001110),
        _ => None,
    }
}

fn prim_00001100(
    args: &[Value],
    env: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    // Довільна арність — за контрактом tests/fixtures/conformance.lisp.
    arithmetic::arithmetic_on_values("+", args, env, span)
}

fn prim_00001101(
    args: &[Value],
    env: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    // Довільна арність — за контрактом tests/fixtures/conformance.lisp.
    arithmetic::arithmetic_on_values("-", args, env, span)
}

fn prim_00001110(
    args: &[Value],
    env: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    // Довільна арність — за контрактом tests/fixtures/conformance.lisp.
    arithmetic::arithmetic_on_values("*", args, env, span)
}

fn prim_00001111(
    args: &[Value],
    env: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    // Довільна арність: (/ 5 6 8 7) -> 5/336 (conformance.lisp).
    arithmetic::division_on_values(args, args.len(), env, span)
}

fn prim_00011010(args: &[Value], _env: &Environment, span: Span) -> Result<Value, LanguageError> {
    arithmetic::comparison_on_values("<", args, span)
}

fn prim_00011011(args: &[Value], _env: &Environment, span: Span) -> Result<Value, LanguageError> {
    arithmetic::comparison_on_values(">", args, span)
}

fn prim_00011100(args: &[Value], _env: &Environment, span: Span) -> Result<Value, LanguageError> {
    arithmetic::comparison_on_values("=", args, span)
}

fn prim_01001101(
    args: &[Value],
    env: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    exact_args(crate::CoreDomainIdentity::D4(crate::CoreD4::from_word(crate::Bit4::new(0b0001).expect("eval"))), args, 1, span)?;
    special_forms::eval_values(args, env, span)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DomainPrimitiveKind {
    StringToSymbol,
    SymbolToString,
    CharCode,
    Read,
    WriteToString,
    Equal,
    AtomPredicate,
    AtomEquality,
    PairConstruct,
}

/// Backend-neutral classification of current exact-domain value primitives.
///
/// Exact domain + exact word select an explicit mechanism; this table never
/// projects a current identity onto historical CoreDomainIdentity/Function8 bytes. The four
/// D9 string operations below are a bounded mechanism cut, not general D9
/// callability: every other D9 resident remains fail-closed.
pub(crate) fn domain_primitive_kind(
    identity: CoreDomainIdentity,
) -> Option<DomainPrimitiveKind> {
    match identity {
        CoreDomainIdentity::D3(word) => match word.word().packed_bits() {
            0b010 => Some(DomainPrimitiveKind::AtomPredicate),
            0b101 => Some(DomainPrimitiveKind::AtomEquality),
            0b111 => Some(DomainPrimitiveKind::PairConstruct),
            0b100 | 0b011 => None,
            _ => None, // QUOTE/COND are syntax routes, 000 is structural empty
        },
        CoreDomainIdentity::D8(word) => match word.word().packed_bits() {
            0b11100000 => Some(DomainPrimitiveKind::CharCode),
            0b11101101 => Some(DomainPrimitiveKind::StringToSymbol),
            0b11101110 => Some(DomainPrimitiveKind::SymbolToString),
            0b11101010 => Some(DomainPrimitiveKind::Read),
            0b11110100 => Some(DomainPrimitiveKind::WriteToString),
            0b11110111 => Some(DomainPrimitiveKind::Equal),
            _ => None,
        },
        _ => None,
    }
}

/// Exact-domain primitive execution never reconstructs an historical SID8.
/// The domain and its exact binary word select the already-admitted mechanism;
/// historical byte dispatch is confined to invoke_semantic_ref below.
fn invoke_domain_primitive(
    identity: CoreDomainIdentity,
    args: &[Value],
    environment: &Environment,
    span: Span,
) -> Option<Result<Value, LanguageError>> {
    let kind = domain_primitive_kind(identity)?;
    Some((|| {
        let expected = match kind {
            DomainPrimitiveKind::StringToSymbol | DomainPrimitiveKind::SymbolToString => 1,
            DomainPrimitiveKind::CharCode => 1,
            DomainPrimitiveKind::Read => args.len(),
            DomainPrimitiveKind::WriteToString => 1,
            DomainPrimitiveKind::AtomPredicate => 1,
            DomainPrimitiveKind::AtomEquality => 2,
            DomainPrimitiveKind::PairConstruct => 2,
            DomainPrimitiveKind::Equal => 2,
        };
        if matches!(kind, DomainPrimitiveKind::Read) && args.len() > 1 {
            return Err(LanguageError::new(
                ErrorKind::Arity,
                format!("{identity}: expected at most 1 argument; received {}", args.len()),
                span,
            ));
        }
        if args.len() != expected {
            return Err(LanguageError::new(
                ErrorKind::Arity,
                format!(
                    "{identity}: expected {expected} arguments; received {}",
                    args.len()
                ),
                span,
            ));
        }
        match kind {
            DomainPrimitiveKind::AtomPredicate => {
                Ok(special_forms::atom_value(&args[0], environment))
            }
            DomainPrimitiveKind::AtomEquality => {
                special_forms::eq_values(args[0].clone(), args[1].clone(), span)
            }
            DomainPrimitiveKind::PairConstruct => {
                special_forms::cons_values(args[0].clone(), args[1].clone(), environment, span)
            }
            DomainPrimitiveKind::Equal => Ok(Value::predicate_bit(args[0] == args[1])),
            DomainPrimitiveKind::StringToSymbol => {
                // D8:11101101, direct scalar/string mechanism, no byte fallback.
                special_forms::string_to_symbol_values(args, span)
            }
            DomainPrimitiveKind::SymbolToString => {
                // D8:11101110, exact inverse carrier with the existing string
                // mechanism. No historical byte/alias is ever consulted.
                special_forms::symbol_to_string_values(args, span)
            }
            DomainPrimitiveKind::CharCode => {
                // D8:11100000 CHAR-CODE reuses the already-tested scalar
                // extraction implementation body. The historical W8 identity
                // is not reconstructed or consulted.
                builtins::prim_01000101(args, environment, span)
            }
            DomainPrimitiveKind::Read => {
                // D8:11101010 owns canonical READ, not the historical SID.
                special_forms::read_values(args, environment, span)
            }
            DomainPrimitiveKind::WriteToString => {
                // D8:11110100 owns WRITE-TO-STRING; call the existing
                // serializer directly, never reconstitute a historical byte.
                special_forms::write_to_string_values(args, environment, span)
            }
        }
    })())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum D9StringPrimitiveKind {
    Empty,
    Rest,
    Prefix,
    First,
}

fn d9_string_primitive_kind(identity: crate::DomainIdentity) -> Option<D9StringPrimitiveKind> {
    let crate::DomainIdentity::D9(word) = identity else {
        return None;
    };
    match word.word().packed_bits() {
        0b110011100 => Some(D9StringPrimitiveKind::Empty),
        0b110011101 => Some(D9StringPrimitiveKind::Rest),
        0b110101010 => Some(D9StringPrimitiveKind::Prefix),
        0b110101011 => Some(D9StringPrimitiveKind::First),
        _ => None,
    }
}

/// True only for the four explicit D9 value mechanisms required by the
/// machine-admission wire checker. This is deliberately outside
/// CoreDomainIdentity: D9 identity remains universal-domain identity, not Core.
pub(crate) fn has_d9_string_mechanism(identity: crate::DomainIdentity) -> bool {
    d9_string_primitive_kind(identity).is_some()
}

/// Execute the bounded D9 string mechanism cut. Routing is selected from the
/// full nine-bit D9 identity; the called functions are reused implementation
/// bodies only, never looked up or invoked through a historical SID8.
pub(crate) fn invoke_d9_string_mechanism(
    identity: crate::DomainIdentity,
    args: &[Value],
    environment: &Environment,
    span: Span,
) -> Option<Result<Value, LanguageError>> {
    let kind = d9_string_primitive_kind(identity)?;
    Some((|| {
        let expected = match kind {
            D9StringPrimitiveKind::Empty | D9StringPrimitiveKind::Rest | D9StringPrimitiveKind::First => 1,
            D9StringPrimitiveKind::Prefix => 2,
        };
        if args.len() != expected {
            return Err(LanguageError::new(
                ErrorKind::Arity,
                format!("D9 {identity}: expected {expected} arguments; received {}", args.len()),
                span,
            ));
        }
        let value = match kind {
            D9StringPrimitiveKind::Empty => {
                super::builtins::prim_00111100(args, environment, span)?
            }
            D9StringPrimitiveKind::Rest => {
                super::builtins::prim_01000000(args, environment, span)?
            }
            D9StringPrimitiveKind::Prefix => {
                super::builtins::prim_00111101(args, environment, span)?
            }
            D9StringPrimitiveKind::First => {
                super::builtins::prim_00111111(args, environment, span)?
            }
        };
        if matches!(kind, D9StringPrimitiveKind::Empty | D9StringPrimitiveKind::Prefix)
            && value.as_predicate_bit().is_none()
        {
            return Err(LanguageError::new(
                ErrorKind::Type,
                format!("D9 {identity} must return an exact D1 PredicateBit; got {value}"),
                span,
            ));
        }
        Ok(value)
    })())
}

/// Canonical value-call mechanism bridge for migrated exact-domain identities.
///
/// The D3 role mapping is explicit and law-shaped; it is intentionally not a
/// numeric projection to the historical Function8 byte axis.
pub(crate) fn has_language_result_boundary(identity: CoreDomainIdentity) -> bool {
    match identity {
        CoreDomainIdentity::D3(word) => matches!(word.word().packed_bits(), 0b010 | 0b101),
        CoreDomainIdentity::D5(word) => matches!(
            word.word().packed_bits(),
            0b01000 | 0b01001 | 0b11010 | 0b11011 | 0b11101
        ),
        CoreDomainIdentity::D8(word) => matches!(word.word().packed_bits(), 0b11110111),
        _ => false,
    }
}

fn canonicalize_domain_result(
    identity: CoreDomainIdentity,
    value: Value,
    span: Span,
) -> Result<Value, LanguageError> {
    if !has_language_result_boundary(identity) || value.as_predicate_bit().is_some() {
        return Ok(value);
    }

    // Контракт 11.8: Rust перевіряє тільки тип межі D1.
    // Старі t/(), числові й спискові відповіді не перетворюються на предикат.
    Err(LanguageError::new(
        ErrorKind::Type,
        format!("exact domain predicate must return D1 PredicateBit; got {value}"),
        span,
    ))
}

/// Compact-derived bootstrap laws ratified by Contract D4: NOT, NULL, LIST, APPEND.
///
/// These are bounded reference value-call mechanisms. NOT consumes and returns
/// only exact D1 PredicateBit values; NULL checks structural EMPTY (Value::Nil).
/// Neither law coerces Number 0/1, host Bool, historical truth, or EMPTY into a
/// predicate. LIST/APPEND use the D3 pair/empty value algebra. These mechanisms
/// do not add residents, host effects, or historical W8 aliases.
fn require_exactly_one_d4_argument(
    coordinate: u8,
    args: &[Value],
    span: Span,
) -> Result<(), LanguageError> {
    if args.len() == 1 {
        return Ok(());
    }
    Err(LanguageError::new(
        ErrorKind::Arity,
        format!(
            "D4:{coordinate:04b} expects exactly one argument / потребує рівно один аргумент; отримано {}",
            args.len()
        ),
        span,
    ))
}

fn invoke_compact_derived_d4(
    identity: CoreDomainIdentity,
    args: &[Value],
    span: Span,
) -> Option<Result<Value, LanguageError>> {
    let CoreDomainIdentity::D4(word) = identity else {
        return None;
    };
    match word.word().packed_bits() {
        // Ratified D4 NOT: exact D1 NO/YES is negated without host truthiness.
        0b0100 => Some((|| {
            require_exactly_one_d4_argument(0b0100, args, span)?;
            let value = args[0].as_predicate_bit().ok_or_else(|| {
                LanguageError::new(
                    ErrorKind::Type,
                    "D4:0100 NOT requires an exact D1 PredicateBit; Number, host Bool, and structural EMPTY are not predicates",
                    span,
                )
            })?;
            Ok(Value::predicate_bit(!value))
        })()),
        // Ratified D4 NULL: only structural empty list is empty; D1 NO is data.
        0b0101 => Some((|| {
            require_exactly_one_d4_argument(0b0101, args, span)?;
            Ok(Value::predicate_bit(matches!(args[0], Value::Nil)))
        })()),
        // Ratified D4 LIST: collect evaluated values into a proper list.
        0b1110 => Some(Ok(Value::list(args.iter().cloned()))),
        // Ratified D4 APPEND: combine only proper lists, left to right.
        // Never coerce a D1 predicate, Text7 datum, symbol, or dotted tail
        // into a proper-list carrier. Each pair is observed, not mutated.
        0b1111 => {
            let mut items = Vec::new();
            for value in args {
                let mut cursor = value;
                loop {
                    match cursor {
                        Value::Nil => break,
                        Value::Pair(head, tail) => {
                            items.push(head.as_ref().clone());
                            cursor = tail.as_ref();
                        }
                        _ => {
                            return Some(Err(LanguageError::new(
                                ErrorKind::Type,
                                "D4:1111 APPEND requires proper lists; no implicit tail or D1 coercion",
                                span,
                            )));
                        }
                    }
                }
            }
            Some(Ok(Value::list(items)))
        }
        _ => None,
    }
}

pub(crate) fn invoke_domain_identity(
    identity: CoreDomainIdentity,
    args: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    if let Some(result) = super::selector_law::invoke(identity, args, span) {
        return result;
    }

    if let Some(result) = invoke_compact_derived_d4(identity, args, span) {
        return result;
    }

    if let Some(result) = super::d5_arithmetic::invoke(identity, args, environment, span) {
        let value = result?;
        return canonicalize_domain_result(identity, value, span);
    }

    if let Some(result) = super::d6_arithmetic::invoke(identity, args, environment, span) {
        return result;
    }

    if let Some(result) = invoke_domain_primitive(identity, args, environment, span) {
        return canonicalize_domain_result(identity, result?, span);
    }

    if let Some(bound) = environment.domain_code_slot(identity) {
        let result = match &bound {
            Value::Closure(closure) => Some(closures::apply_values(closure.clone(), args, span)?),
            Value::Builtin(builtin) => Some((builtin.func)(args, environment, span)?),
            _ => None,
        };
        if let Some(value) = result {
            return canonicalize_domain_result(identity, value, span);
        }
    }

    Err(LanguageError::new(
        ErrorKind::Type,
        format!("domain identity has no admitted value-call mechanism: {identity}"),
        span,
    ))
}

/// Compatibility/backend mechanism bridge selected by historical CoreDomainIdentity.
pub(crate) fn invoke_semantic_ref(
    sid: CoreDomainIdentity,
    args: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    // Transitional input adapter only: the five historical Canon value calls
    // now execute the ratified D3 mechanism, never duplicate Rust SID8 bodies.
    // No historical byte gains placement authority; an unmapped SID fails closed.
    if let Some(identity) = semantic_registry::compatibility_d3_route_from_ratified_domains(sid) {
        if matches!(route_kind_for_sid(sid), Some(SidRouteKind::ValueCall)) {
            return invoke_domain_identity(identity, args, environment, span);
        }
    }
    // Unmigrated non-D3 host capabilities remain on the temporary backend lane.
    if let Some(primitive) = legacy_backend_primitive(sid) {
        return primitive(args, environment, span);
    }
    match &environment.code_slot(sid) {
        Some(Value::Closure(closure)) => return closures::apply_values(closure.clone(), args, span),
        Some(Value::Builtin(builtin)) => return (builtin.func)(args, environment, span),
        _ => {}
    }

    if let Some(profile) = environment.selected_core_profile() {
        if matches!(
            profile_mechanism_route(profile, sid),
            Some(ProfileMechanismRouteKind::RegisteredHostMechanism)
        ) {
            return match super::capabilities::dispatch_sens_capability(
                sid,
                args,
                environment,
                span,
            ) {
                Some(result) => result,
                None => Err(LanguageError::new(
                    ErrorKind::MechanismUnavailable,
                    format!("admitted host mechanism is unavailable for SENS function: {sid}"),
                    span,
                )),
            };
        }
    }

    Err(LanguageError::new(
        ErrorKind::Type,
        format!("SENS function has no admitted callable mechanism: {sid}"),
        span,
    ))
}

/// #1455: чи має код примітив Rust.
pub(crate) fn has_primitive(sid: CoreDomainIdentity) -> bool {
    // D3 is protected by the current exact-domain mechanism even when the
    // caller still arrives through a historical, read-only compatibility SID.
    matches!(route_kind_for_sid(sid), Some(SidRouteKind::ValueCall))
        || legacy_backend_primitive(sid).is_some()
}

/// #1455: визначення (функція або макрос) верхнього рівня з назвою з таблиці функцій, чий код не
/// має примітиву й не є особливою формою, стає механізмом цього коду. Лише
/// перше визначення; затінення назви пізніше слот не змінює.
pub(crate) fn bind_language_definition(name: &str, value: &Value, environment: &Environment) {
    if !environment.is_root() || !matches!(value, Value::Closure(_) | Value::Builtin(_) | Value::Macro(_)) {
        return;
    }

    if let Some(identity) = semantic_registry::d9_binding_identity_for_definition(name) {
        // Concrete Lisp implementation only; D9 itself remains non-Core.
        environment.bind_domain_code_slot_once(identity, value.clone());
    }

    if let Some(identity) = semantic_registry::d6_binding_identity_for_definition(name) {
        // D6 residency remains separate from execution. Only an actual
        // language-owned callable definition can populate this exact slot.
        if identity.core_operation().is_none() {
            environment.bind_domain_code_slot_once(identity, value.clone());
        }
    }

    let direct_d5_binding = semantic_registry::d5_binding_identity_for_definition(name);
    if let Some(identity) = direct_d5_binding {
        if super::necessary_forms::identity_for_domain_identity(identity).is_none()
            && domain_primitive_kind(identity).is_none()
            && !super::d5_arithmetic::has_mechanism(identity)
        {
            environment.bind_domain_code_slot_once(identity, value.clone());
        }
    }

    if let Some(identity) = semantic_registry::domain_identity_for_surface(name) {
        if domain_primitive_kind(identity).is_some()
            || super::necessary_forms::identity_for_domain_identity(identity).is_some()
        {
            return;
        }
        environment.bind_domain_code_slot_once(identity, value.clone());

        // D3/D4 direct-domain definitions are fully migrated and stop here.
        // Lisp-owned D5 definitions still have historical callers inside the
        // compatibility Core/library corpus, so they must continue below long
        // enough to install the temporary alias to this same Value/Rc.
        if direct_d5_binding.is_none() {
            return;
        }
    }

    // Compatibility-only lane for registry rows that do not yet have a
    // canonical domain identity.
    let Some(sid) = semantic_registry::admitted_semantic_id_for_surface(name) else {
        return;
    };
    // No byte-to-domain lookup is permitted here. The only D5 identity
    // originates in the generated, domain-qualified Lisp definition bindings
    // above; the SID slot is merely a temporary alias for unmigrated callers.
    if let Some(identity) = direct_d5_binding {
        if super::necessary_forms::identity_for_domain_identity(identity).is_some() {
            return;
        }
        environment.bind_code_slot_once(sid, value.clone());
        return;
    }

    if has_primitive(sid) || super::necessary_forms::identity_for_semantic_id(sid).is_some() {
        return;
    }
    environment.bind_code_slot_once(sid, value.clone());
}


#[cfg(test)]
mod exact_domain_primitive_tests {
    use super::*;

    #[test]
    fn current_d3_atom_surface_is_reserved_without_legacy_sid8_routing() {
        let identity = immutable_exact_surface_identity("атом?")
            .expect("D3 atom must have a ratified exact-domain surface");
        assert!(matches!(identity, CoreDomainIdentity::D3(_)));
        assert!(is_reserved_surface("атом?"));
        assert!(
            ensure_bindable("атом?", Span { start: 0, end: 0 }).is_err(),
            "ratified D3 surface must never be rebound through an old byte alias"
        );
    }

    fn d3(bits: u8) -> CoreDomainIdentity {
        CoreDomainIdentity::D3(crate::Bija3::from_word(
            crate::Bit3::new(bits).expect("D3 word"),
        ))
    }

    fn d4(bits: u8) -> CoreDomainIdentity {
        CoreDomainIdentity::D4(crate::CoreD4::from_word(
            crate::Bit4::new(bits).expect("D4 word"),
        ))
    }

    #[test]
    fn d4_not_negates_only_exact_d1_predicate_bits() {
        let environment = Environment::root();
        let span = Span { start: 0, end: 0 };

        for (input, expected) in [(false, true), (true, false)] {
            let result = invoke_domain_identity(
                d4(0b0100),
                &[Value::predicate_bit(input)],
                &environment,
                span,
            )
            .expect("ratified D4 NOT has a compact value-call mechanism");
            assert_eq!(result.as_predicate_bit(), Some(expected));
        }

        for invalid in [
            Value::Number(0.0, crate::Exactness::Exact),
            Value::Number(1.0, crate::Exactness::Exact),
            Value::Nil,
            Value::Bool(false),
        ] {
            let error = invoke_domain_identity(d4(0b0100), &[invalid], &environment, span)
                .expect_err("D4 NOT must reject non-D1 values without coercion");
            assert_eq!(error.kind, ErrorKind::Type);
        }

        let arity = invoke_domain_identity(d4(0b0100), &[], &environment, span)
            .expect_err("NOT arity remains one");
        assert_eq!(arity.kind, ErrorKind::Arity);
    }

    #[test]
    fn d4_null_tests_structural_empty_not_d1_no() {
        let environment = Environment::root();
        let span = Span { start: 0, end: 0 };

        for (value, expected) in [
            (Value::Nil, true),
            (Value::predicate_bit(false), false),
            (Value::predicate_bit(true), false),
            (Value::Number(0.0, crate::Exactness::Exact), false),
            (Value::Number(1.0, crate::Exactness::Exact), false),
            (Value::list([Value::Nil]), false),
        ] {
            let result = invoke_domain_identity(d4(0b0101), &[value], &environment, span)
                .expect("ratified D4 NULL has a compact value-call mechanism");
            assert_eq!(result.as_predicate_bit(), Some(expected));
        }

        let arity = invoke_domain_identity(d4(0b0101), &[], &environment, span)
            .expect_err("NULL arity remains one");
        assert_eq!(arity.kind, ErrorKind::Arity);
    }

    fn d9(bits: u16) -> crate::DomainIdentity {
        crate::DomainIdentity::D9(crate::CoreD9::from_word(
            crate::Bit9::new(bits).expect("D9 word"),
        ))
    }

    #[test]
    fn four_ratified_d9_string_operations_have_explicit_exact_domain_mechanisms() {
        let environment = Environment::root();
        let span = Span { start: 0, end: 0 };

        let empty = invoke_d9_string_mechanism(
            d9(0b110011100),
            &[Value::String(std::rc::Rc::from(""))],
            &environment,
            span,
        ).expect("D9 STRING-EMPTY? route").expect("D9 STRING-EMPTY? value");
        assert_eq!(empty.as_predicate_bit(), Some(true));

        let nonempty = invoke_d9_string_mechanism(
            d9(0b110011100),
            &[Value::String(std::rc::Rc::from("q2:7/1"))],
            &environment,
            span,
        ).expect("D9 STRING-EMPTY? route").expect("D9 STRING-EMPTY? value");
        assert_eq!(nonempty.as_predicate_bit(), Some(false));

        let starts = invoke_d9_string_mechanism(
            d9(0b110101010),
            &[
                Value::String(std::rc::Rc::from("#q2:")),
                Value::String(std::rc::Rc::from("#q2:7/1")),
            ],
            &environment,
            span,
        ).expect("D9 STRING-PREFIX? route").expect("D9 STRING-PREFIX? value");
        assert_eq!(starts.as_predicate_bit(), Some(true));

        let does_not_start = invoke_d9_string_mechanism(
            d9(0b110101010),
            &[
                Value::String(std::rc::Rc::from("q2:")),
                Value::String(std::rc::Rc::from("#q2:7/1")),
            ],
            &environment,
            span,
        ).expect("D9 STRING-PREFIX? route").expect("D9 STRING-PREFIX? value");
        assert_eq!(does_not_start.as_predicate_bit(), Some(false));

        let first = invoke_d9_string_mechanism(
            d9(0b110101011),
            &[Value::String(std::rc::Rc::from("#q2:7/1"))],
            &environment,
            span,
        ).expect("D9 STRING-FIRST route").expect("D9 STRING-FIRST value");
        assert!(matches!(first, Value::String(ref value) if value.as_ref() == "#"));

        let rest = invoke_d9_string_mechanism(
            d9(0b110011101),
            &[Value::String(std::rc::Rc::from("#q2:7/1"))],
            &environment,
            span,
        ).expect("D9 STRING-REST route").expect("D9 STRING-REST value");
        assert!(matches!(rest, Value::String(ref value) if value.as_ref() == "q2:7/1"));

        assert!(!has_d9_string_mechanism(d9(0b110111111)),
            "other D9 residents stay fail-closed");
        assert!(!has_d9_string_mechanism(crate::DomainIdentity::D8(
            crate::CoreD8::from_word(crate::Bit8::new(0b00111100).expect("D8 word"))
        )), "D8 coordinates never alias D9 mechanisms");

        let arity_error = invoke_d9_string_mechanism(
            d9(0b110101010),
            &[Value::String(std::rc::Rc::from("#q2:"))],
            &environment,
            span,
        ).expect("D9 STRING-PREFIX? route")
         .expect_err("exact two-argument arity");
        assert_eq!(arity_error.kind, ErrorKind::Arity);
        assert!(arity_error.message.contains(&d9(0b110101010).to_string()));
        assert!(!arity_error.message.contains("00111101"));
    }

    #[test]
    fn legacy_backend_is_sparse_and_never_autocreates_current_domain_calls() {
        let expected = [
            0b00001100,
            0b00001101,
            0b00001110,
            0b00001111,
            0b00011010,
            0b00011011,
            0b00011100,
            0b01001101,
            0b00111011,
            0b00111100,
            0b00111101,
            0b00111110,
            0b01010000,
            0b01001111,
            0b01011010,
            0b01011011,
            0b01011100,
            0b01011101,
            0b01010001,
            0b01010010,
            0b01010011,
            0b01010100,
            0b01010101,
            0b01000001,
            0b00111010,
            0b00100100,
            0b01000010,
            0b01000011,
            0b00111111,
            0b01000000,
            0b01000100,
            0b01000101,
            0b10100001,
            0b10100000,
            0b01001000,
            0b01001001,
            0b01001100,
            0b01001010,
            0b01001011,
            0b00100110,
            0b01010110,
            0b01010111,
            0b01011000,
            0b01011001,
            0b01001110,
        ];
        assert_eq!(expected.len(), 45);
        for candidate in 0..=u8::MAX {
            let routed = legacy_backend_primitive(CoreDomainIdentity::from_packed_byte(candidate)).is_some();
            assert_eq!(
                routed,
                expected.contains(&candidate),
                "сумісний backend допускає лише явно збережені історичні механізми"
            );
        }
        // Та сама числова величина у іншій доменній ширині — інша
        // ідентичність; D3 механізмів у старому backend не існує.
        for bits in 0..=7 {
            let exact = d3(bits);
            assert_eq!(exact.width(), 3);
            assert!(legacy_backend_primitive(CoreDomainIdentity::from_packed_byte(bits)).is_none());
        }
    }

    #[test]
    fn d3_uses_one_domain_mechanism_without_duplicate_byte_primitives() {
        // The historical Canon byte slots have no independent primitive
        // implementation; all current D3 operations use domain laws.
        for old_byte in 2..7 {
            assert!(
                legacy_backend_primitive(CoreDomainIdentity::from_packed_byte(old_byte)).is_none(),
                "історичний байт D3 не отримує дубльований примітив"
            );
        }
        let environment = Environment::root();
        let span = Span { start: 0, end: 0 };
        let arity = invoke_domain_identity(d3(0b010), &[], &environment, span)
            .expect_err("D3 ATOM must enforce current exact arity");
        assert_eq!(arity.kind, ErrorKind::Arity);
        assert!(arity.message.contains("010: expected 1 arguments; received 0"));
    }

    #[test]
    fn domain_primitive_arity_diagnostic_carries_domain_not_historical_sid() {
        let environment = Environment::root();
        let span = Span { start: 0, end: 0 };
        let identity = d3(0b010);
        let error = invoke_domain_primitive(identity, &[], &environment, span)
            .expect("D3 ATOM admitted")
            .expect_err("arity mismatch must fail");
        assert_eq!(error.kind, ErrorKind::Arity);
        assert!(error.message.contains(&identity.to_string()));
        assert!(!error.message.contains("00000010"));
    }
}
