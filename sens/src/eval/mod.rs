//! Evaluator entry points and the special-form dispatcher.
//! Tochky vkhodu evaluator i dyspetcher spetsialnykh form.
//! Einstiegspunkte des Evaluators und der Sonderformen-Dispatcher.
//!
//! The evaluator is split by concern: this module owns the trampoline loop and
//! dispatch table, `arithmetic` owns exact/inexact number handling, `special_forms`
//! owns the McCarthy primitives plus compatibility `def`/`cond`,
//! `necessary_forms` owns the immutable DEFINE/LAMBDA identities, and `closures`
//! owns lambda construction and function/macro application.
pub(crate) use special_forms::digest::sha256 as digest_sha256;

mod arithmetic;
mod d5_arithmetic;
mod d6_arithmetic;
pub(crate) mod builtins;
pub(crate) mod canon;
mod capabilities;
mod closures;
pub(crate) mod lower;
mod macro_substrate;
pub(crate) mod necessary_forms;
mod profile_mechanisms_generated;
pub(crate) mod selector_law;
mod special_forms;

pub use capabilities::{
    capability_installed, installed_capabilities, register_capability,
    register_evaluated_capability, register_sens_capability, unregister_capability,
    unregister_sens_capability,
};
pub(crate) use macro_substrate::install as install_macro_substrate;
pub use special_forms::{exact_arity, json::parse_json};

use crate::{parse, Environment, ErrorKind, Expr, ExprKind, LanguageError, Session, CoreDomainIdentity, Span, Value};
use crate::CoreDomainIdentity;
use crate::canonical_reader::text7_binding_key;

#[derive(Clone, Debug, PartialEq)]
pub struct EvalResult {
    pub value: Value,
    pub output: Vec<String>,
}

pub fn eval_parsed_expressions(
    expressions: &[Expr],
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    eval_lowered_expressions(&lower::lower_program(expressions), session)
}

/// Виконати програму, вже зведену `lower_program`.
/// Мігрувані D3/D4 голови несуть exact domain identity; неперенесені
/// compatibility-голови ще можуть нести historical CoreDomainIdentity.
/// Прив'язати безадресну внутрішню помилку до виконуваної верхньої форми.
/// Точні діапазони помилок зберігаються; семантика виконання не змінюється.
fn top_level_error_span(mut error: LanguageError, expression: &Expr) -> LanguageError {
    if error.span == Span::default() {
        error.span = expression.span;
    }
    error
}

pub fn eval_lowered_expressions(
    expressions: &[Expr],
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    let mut value = Value::Nil;
    for expression in expressions {
        value = evaluate(expression, &session.environment)
            .map_err(|error| top_level_error_span(error, expression))?;
    }
    Ok(EvalResult {
        value,
        output: session.environment.output_snapshot(),
    })
}

pub fn eval_parsed_expressions_incremental(
    expressions: &[Expr],
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    session.environment.output_take_new();
    let mut value = Value::Nil;
    for expression in lower::lower_program(expressions).iter() {
        value = evaluate(expression, &session.environment)?;
    }
    Ok(EvalResult {
        value,
        output: session.environment.output_take_new(),
    })
}

pub fn eval_program(source: &str, session: &mut Session) -> Result<EvalResult, LanguageError> {
    let expressions = parse(source)?;
    eval_parsed_expressions(&expressions, session)
}

/// Fail closed before the first side effect of an exact-domain program.
///
/// Compatibility-only surface names are not admitted Dn call heads; otherwise
/// a raw List could reach the evaluator's historical symbol dispatcher.
/// Traversal is iterative and skips D3 quoted data and D4 binder positions.
pub(crate) fn validate_exact_executable_heads(expressions: &[Expr]) -> Result<(), LanguageError> {
    let mut pending: Vec<&Expr> = expressions.iter().collect();
    while let Some(expr) = pending.pop() {
        match &expr.kind {
            ExprKind::Sid(_) | ExprKind::Call(_, _) => {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    "exact-domain program cannot contain historical W8 AST operations",
                    expr.span,
                ));
            }
            ExprKind::List(items) => {
                if let Some(Expr { kind: ExprKind::Symbol(head), span }) = items.first() {
                    if crate::semantic_registry::admitted_semantic_id_for_surface(head).is_some()
                        && crate::semantic_registry::domain_identity_for_surface(head).is_none()
                    {
                        return Err(LanguageError::new(
                            ErrorKind::InvalidForm,
                            format!("historical-only call surface is not a ratified Dn head: {head}"),
                            *span,
                        ));
                    }
                }
                pending.extend(items.iter());
            }
            ExprKind::DomainCall(identity, args) if is_d3(*identity, 0b001) => {
                // D3 QUOTE stores literal syntax, including symbols which are
                // intentionally not executable heads.
                let _ = args;
            }
            ExprKind::DomainCall(identity, args)
                if necessary_forms::identity_for_domain_identity(*identity).is_some() =>
            {
                // First D4 LAMBDA/DEFINE argument is a binder, not a call.
                pending.extend(args.iter().skip(1));
            }
            ExprKind::DomainCall(_, args) => pending.extend(args.iter()),
            ExprKind::Pair(head, tail) => {
                pending.push(head);
                pending.push(tail);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Виконати точне джерело D1–D9 без історичного восьмибітного читача.
///
/// Ширина слова зберігається у DomainIdentity. Резидентність не надає права
/// виклику; перевірка механізму відбувається в доменному evaluator-і.
/// Літерали чисел D24+ не допускаються цим читачем до окремого HANDOFF.
pub fn eval_exact_domain_program(
    source: &str,
    session: &mut Session,
) -> Result<EvalResult, LanguageError> {
    let expressions = crate::parser::parse_exact_domain_source(source)?;
    // Never pass canonical Dn source through historical W8 head lowering,
    // or permit a legacy-only textual head to reach the compatibility evaluator.
    let lowered = lower::lower_exact_domain_program(&expressions);
    validate_exact_executable_heads(&lowered)?;
    eval_lowered_expressions(&lowered, session)
}

pub(crate) enum EvalStep {
    Value(Value),
    TailCall {
        expression: Expr,
        environment: Environment,
    },
}

pub(crate) fn invoke_value(
    function: &Value,
    arguments: &[Value],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    match function {
        Value::DomainIdentity(identity) => {
            if let Some(core_identity) = identity.core_operation() {
                return canon::invoke_domain_identity(core_identity, arguments, environment, span);
            }
            // A bounded D9 mechanism cut remains outside CoreDomainIdentity.
            if canon::has_d9_string_mechanism(*identity) {
                return canon::invoke_d9_string_mechanism(*identity, arguments, environment, span)
                    .expect("exact D9 string-mechanism route was checked above");
            }
            match environment.domain_code_slot(*identity).as_ref() {
                Some(Value::Builtin(builtin)) => (builtin.func)(arguments, environment, span),
                Some(Value::Closure(closure)) => closures::apply_values(closure.clone(), arguments, span),
                _ => Err(LanguageError::new(
                    ErrorKind::Type,
                    format!(
                        "domain identity is not callable under its ratified law: D{} {}",
                        identity.width(),
                        identity
                    ),
                    span,
                )),
            }
        }
        Value::Sid(sid) => canon::invoke_semantic_ref(*sid, arguments, environment, span),
        Value::Builtin(builtin) => (builtin.func)(arguments, environment, span),
        Value::Closure(closure) => closures::apply_values(closure.clone(), arguments, span),
        _ => Err(LanguageError::new(
            ErrorKind::Type,
            format!(
                "expression is not callable · vyraz ne mozhna vyklykaty · Ausdruck ist nicht aufrufbar (носій: {})",
                match function {
                    Value::Nil => "порожня структура",
                    Value::Bool(_) => "історичний булевий носій",
                    Value::Number(_, _) | Value::Rational(_) => "число",
                    Value::BinaryNumber(_) => "двійкове число",
                    Value::Symbol(_) => "символ",
                    Value::Pair(_, _) => "пара",
                    Value::Macro(_) => "макрос",
                    Value::String(_) | Value::Text7(_) => "текст",
                    _ => "інший невикликний носій",
                },
            ),
            span,
        )),
    }
}

pub fn evaluate(expression: &Expr, environment: &Environment) -> Result<Value, LanguageError> {
    let (mut owned_expression, mut owned_environment) =
        match evaluate_step(expression, environment)? {
            EvalStep::Value(value) => return Ok(value),
            EvalStep::TailCall {
                expression,
                environment,
            } => (expression, environment),
        };

    loop {
        match evaluate_step(&owned_expression, &owned_environment)? {
            EvalStep::Value(value) => return Ok(value),
            EvalStep::TailCall {
                expression: next,
                environment: next_environment,
            } => {
                owned_expression = next;
                owned_environment = next_environment;
            }
        }
    }
}

pub(crate) fn evaluate_step(
    expression: &Expr,
    environment: &Environment,
) -> Result<EvalStep, LanguageError> {
    match &expression.kind {
        ExprKind::Number(number, exactness) => Ok(EvalStep::Value(Value::Number(*number, *exactness))),
        ExprKind::Rational(rational) => Ok(EvalStep::Value(Value::Rational(rational.clone()))),
        ExprKind::BinaryNumber(number) => Ok(EvalStep::Value(Value::BinaryNumber(number.clone()))),
        ExprKind::Sid(sid) => Ok(EvalStep::Value(Value::Sid(*sid))),
        ExprKind::DomainIdentity(identity) => {
            Ok(EvalStep::Value(Value::DomainIdentity(*identity)))
        }
        ExprKind::NumericBuffer(buffer) => Ok(EvalStep::Value(Value::NumericBuffer(buffer.clone()))),
        ExprKind::String(value) => Ok(EvalStep::Value(Value::String(value.clone()))),
        ExprKind::Symbol(symbol) => {
            // A ratified D3 surface is syntax-only here. Diagnose its exact
            // domain identity, not the historical SID8 compatibility byte.
            if let Some(identity @ CoreDomainIdentity::D3(_)) =
                lower::immutable_surface_domain_identity(symbol)
            {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!("exact domain surface is syntax-only in this position: D3 {identity}"),
                    expression.span,
                ));
            }
            if let Some(sid) = canon::routed_sid_for_surface(symbol) {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!(
                        "function SID is syntax-only in this position · function SID тут лише синтаксис · Funktions-SID ist hier nur Syntax: {sid}"
                    ),
                    expression.span,
                ));
            }
            if necessary_forms::identity_for_symbol(symbol).is_some() {
                return Err(LanguageError::new(
                    ErrorKind::UnknownSymbol,
                    format!("evaluator-owned necessary form is syntax-only: {symbol}"),
                    expression.span,
                ));
            }
            if let Some(value) = environment.get(symbol) {
                return Ok(EvalStep::Value(value));
            }
            Err(LanguageError::new(
                ErrorKind::UnknownSymbol,
                format!("unknown symbol · nevidomyi symvol · unbekanntes Symbol: {symbol}"),
                expression.span,
            ))
        }
        // Empty structure is a structural value, not any function SID.
        ExprKind::List(items) if items.is_empty() => Ok(EvalStep::Value(Value::Nil)),
        ExprKind::List(items) => {
            // A whole D2/W7 frame is a Text7 identifier only when the
            // executable context already has the corresponding binding.
            // This is the value-reference case for local/global variables;
            // quoted/data lists never reach here as executable references.
            if let Some(key) = text7_binding_key(expression) {
                if let Some(value) = environment.get(&key) {
                    return Ok(EvalStep::Value(value));
                }
            }
            evaluate_list(items, environment, expression.span)
        },
        ExprKind::Call(sid, arguments) => {
            dispatch_call(None, Some(*sid), None, arguments, environment, expression.span)
        }
        ExprKind::DomainCall(identity, arguments) => {
            dispatch_domain_call(*identity, arguments, environment, expression.span)
        }
        // Виконання залежить лише від числових координат (#1697): слот або є,
        // або названа помилка — пошуку за іменем більше немає.
        ExprKind::Local { depth, index } => environment
            .get_local(*depth, *index)
            .map(EvalStep::Value)
            .ok_or_else(|| {
                LanguageError::new(
                    ErrorKind::UnknownSymbol,
                    format!(
                        "lexical slot {depth}.{index} is not bound in this frame · leksychnyi slot {depth}.{index} ne zv'iazanyi u tsomu freimi · lexikalischer Slot {depth}.{index} ist in diesem Frame nicht gebunden"
                    ),
                    expression.span,
                )
            }),
        ExprKind::Pair(_, _) => Err(LanguageError::new(
            ErrorKind::InvalidForm,
            "a dotted pair is not executable code · dotted-para ne ye vykonuvanym kodom · ein Dotted Pair ist kein ausführbarer Code",
            expression.span,
        )),
    }
}

fn evaluate_list(
    items: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    if let Some(identity) = binary_head_domain_identity(&items[0]) {
        return dispatch_domain_call(identity, &items[1..], environment, span);
    }
    // A D2/W7 frame becomes a Text7 binding only in executable call-head
    // position. Ordinary D2 lists, quoted data and dotted data remain governed
    // solely by the canonical D2 reader.
    if let Some(key) = text7_binding_key(&items[0]) {
        let function = environment.get(&key).ok_or_else(|| {
            LanguageError::new(
                ErrorKind::UnknownSymbol,
                format!("unknown Text7 binding: {key}"),
                items[0].span,
            )
        })?;
        return closures::apply(function, &items[1..], environment, span);
    }
    dispatch_call(
        items[0].kind.as_symbol(),
        binary_head_sid(&items[0]),
        Some(&items[0]),
        &items[1..],
        environment,
        span,
    )
}

fn is_d3(identity: CoreDomainIdentity, bits: u8) -> bool {
    matches!(
        identity,
        CoreDomainIdentity::D3(word) if word.word().packed_bits() == bits
    )
}

fn is_d5(identity: CoreDomainIdentity, bits: u8) -> bool {
    matches!(
        identity,
        CoreDomainIdentity::D5(word) if word.word().packed_bits() == bits
    )
}

fn evaluate_d5_label(
    arguments: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    special_forms::exact_arity("D5:00100", arguments, 2, span)?;

    // LABEL accepts a name in the canonical D2/D7 binding context. Keep
    // the same Text7 wire key used by variable lookup and LAMBDA parameter
    // binding; do not stringify D7 bits as human names or introduce a SID map.
    let name = if let Some(name) = text7_binding_key(&arguments[0]) {
        canon::ensure_bindable(&name, arguments[0].span)?;
        name
    } else {
        match &arguments[0].kind {
            ExprKind::Symbol(name) => {
                canon::ensure_bindable(name, arguments[0].span)?;
                name.clone()
            }
            _ => {
                return Err(LanguageError::new(
                    ErrorKind::Type,
                    "D5:00100 LABEL name must be a literal symbol or D2-framed D7 Text7 binder",
                    arguments[0].span,
                ));
            }
        }
    };

    // LABEL is local self-reference, not global DEFINE.  The closure captures
    // this child frame; replacing the temporary cell after construction makes
    // recursive calls observe the closure itself through the same Rc-backed
    // environment without adding a second recursion mechanism.
    let recursive_environment = environment.child();
    recursive_environment.define(name.clone(), Value::Nil);
    let value = evaluate(&arguments[1], &recursive_environment)?;
    if !matches!(value, Value::Closure(_)) {
        return Err(LanguageError::new(
            ErrorKind::Type,
            "D5:00100 LABEL value must evaluate to a function closure",
            arguments[1].span,
        ));
    }
    recursive_environment.define(name, value.clone());
    Ok(EvalStep::Value(value))
}

fn evaluate_d5_set_family(
    identity: CoreDomainIdentity,
    arguments: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    let is_set = is_d5(identity, 0b00110);
    let is_setq = is_d5(identity, 0b00111);
    debug_assert!(is_set || is_setq);

    let label = if is_set { "D5:00110" } else { "D5:00111" };
    special_forms::exact_arity(label, arguments, 2, span)?;

    let target = if is_set {
        match evaluate(&arguments[0], environment)? {
            Value::Symbol(ref name) => name.clone(),
            _ => {
                return Err(LanguageError::new(
                    ErrorKind::Type,
                    format!("{label} target must evaluate to a symbol"),
                    arguments[0].span,
                ));
            }
        }
    } else {
        match &arguments[0].kind {
            ExprKind::Symbol(name) => name.clone(),
            _ => {
                return Err(LanguageError::new(
                    ErrorKind::Type,
                    format!("{label} target must be a literal symbol"),
                    arguments[0].span,
                ));
            }
        }
    };
    let value = evaluate(&arguments[1], environment)?;
    if !environment.update_nearest_existing(&target, value.clone()) {
        return Err(LanguageError::new(
            ErrorKind::UnknownSymbol,
            format!("{label} target is not an existing binding: {target}"),
            arguments[0].span,
        ));
    }

    Ok(EvalStep::Value(value))
}

fn dispatch_domain_call(
    identity: CoreDomainIdentity,
    arguments: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    if is_d3(identity, 0b001) {
        special_forms::exact_arity("D3:001", arguments, 1, span)?;
        return special_forms::quoted(&arguments[0]).map(EvalStep::Value);
    }

    match necessary_forms::identity_for_domain_identity(identity) {
        Some(necessary_forms::NecessaryFormIdentity::Lambda) => {
            return closures::create_lambda(arguments, environment, span).map(EvalStep::Value);
        }
        Some(necessary_forms::NecessaryFormIdentity::Define) => {
            return special_forms::evaluate_definition(arguments, environment, span)
                .map(EvalStep::Value);
        }
        None => {}
    }

    if is_d3(identity, 0b110) {
        return special_forms::evaluate_domain_cond(arguments, environment, span);
    }

    if is_d5(identity, 0b00100) {
        return evaluate_d5_label(arguments, environment, span);
    }

    if is_d5(identity, 0b00110) || is_d5(identity, 0b00111) {
        return evaluate_d5_set_family(identity, arguments, environment, span);
    }

    if let Some(bound) = environment.domain_code_slot(identity) {
        match &bound {
            Value::Macro(closure) => {
                return closures::apply_macro(closure.clone(), arguments, environment, span);
            }
            Value::Closure(_) if !canon::has_language_result_boundary(identity) => {
                return closures::apply(bound.clone(), arguments, environment, span);
            }
            // #3060: exact D5 MEMBER keeps the same Lisp closure/search law,
            // but its canonical result must cross the D1 PredicateBit boundary.
            // Fall through to evaluated-value invocation so canon can normalize
            // only the final result carrier; every other closure keeps the
            // ordinary tail-call fast path above.
            Value::Closure(_) => {}
            _ => {}
        }
    }

    let mut values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        values.push(evaluate(argument, environment)?);
    }
    canon::invoke_domain_identity(identity, &values, environment, span).map(EvalStep::Value)
}

/// Спільний compatibility-диспетчер виклику. Для `ExprKind::Call` ім'я голови відсутнє:
/// функція — лише 1 байт `head_sid`.
fn dispatch_call(
    head_name: Option<&str>,
    head_sid: Option<CoreDomainIdentity>,
    head_expr: Option<&Expr>,
    arguments: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    let routed_head_sid = head_sid
        .filter(|sid| canon::route_kind_for_sid(*sid).is_some())
        .or_else(|| head_name.and_then(canon::routed_sid_for_surface));
    // Ratified D4 source surfaces take the exact-domain evaluator path,
    // including historical spelling aliases already resolved by the owner
    // registry. Only a genuinely historical W8 head retains the legacy route.
    // Never infer a current D8 identity from an eight-bit payload.
    if let Some(identity) = head_name.and_then(necessary_forms::domain_identity_for_symbol) {
        return dispatch_domain_call(identity, arguments, environment, span);
    }
    let necessary_head = head_sid.and_then(necessary_forms::identity_for_semantic_id);

    // Історичні W8 QUOTE/COND не утворюють незалежного закону виконання:
    // лише засвідчена українська/санскритська проєкція реєстру може
    // направити сумісний виклик до справжнього D3. Однаковий payload
    // інших W8 і D8 не допускає такого переходу.
    let compatible_d3 = routed_head_sid
        .and_then(crate::semantic_registry::compatibility_d3_route_from_ratified_domains);
    if let Some(identity) = compatible_d3 {
        if is_d3(identity, 0b001) || is_d3(identity, 0b110) {
            return dispatch_domain_call(identity, arguments, environment, span);
        }
    }
    if necessary_head == Some(necessary_forms::NecessaryFormIdentity::Lambda) {
        return closures::create_lambda(arguments, environment, span).map(EvalStep::Value);
    }
    if necessary_head == Some(necessary_forms::NecessaryFormIdentity::Define) {
        return special_forms::evaluate_definition(arguments, environment, span).map(EvalStep::Value);
    }

    if let Some(name) = head_name {
        if let Some(result) =
            capabilities::dispatch_capability(name, arguments, environment, span)
        {
            return result;
        }
    }
    let function = match head_sid {
        Some(sid) => Value::Sid(sid),
        None => evaluate(
            head_expr.expect("a call without a SID head keeps its head expression"),
            environment,
        )?,
    };
    match &function {
        Value::DomainIdentity(identity) => {
            if let Some(core_identity) = identity.core_operation() {
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    values.push(evaluate(argument, environment)?);
                }
                return canon::invoke_domain_identity(core_identity, &values, environment, span)
                    .map(EvalStep::Value);
            }
            // Only four exact D9 string residents have this direct value-call
            // mechanism. Leave all other D9 values on the ordinary fail-closed slot path.
            if canon::has_d9_string_mechanism(*identity) {
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    values.push(evaluate(argument, environment)?);
                }
                return canon::invoke_d9_string_mechanism(*identity, &values, environment, span)
                    .expect("exact D9 string-mechanism route was checked above")
                    .map(EvalStep::Value);
            }
            let Some(bound) = environment.domain_code_slot(*identity) else {
                return Err(LanguageError::new(
                    ErrorKind::Type,
                    format!(
                        "domain identity is not callable under its ratified law: D{} {}",
                        identity.width(),
                        identity
                    ),
                    span,
                ));
            };
            match &bound {
                Value::Macro(closure) => closures::apply_macro(closure.clone(), arguments, environment, span),
                Value::Closure(_) => closures::apply(bound.clone(), arguments, environment, span),
                Value::Builtin(builtin) => {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        values.push(evaluate(argument, environment)?);
                    }
                    (builtin.func)(&values, environment, span).map(EvalStep::Value)
                }
                _ => Err(LanguageError::new(
                    ErrorKind::Type,
                    format!("exact domain mechanism slot is not callable: D{} {}", identity.width(), identity),
                    span,
                )),
            }
        }
        Value::Sid(sid) => {
            // #1455: макрос, прив'язаний до коду, розгортається до обчислення аргументів.
            if !canon::has_primitive(*sid) {
                match &environment.code_slot(*sid) {
                    Some(Value::Macro(closure)) => {
                        return closures::apply_macro(closure.clone(), arguments, environment, span);
                    }
                    // A language-defined function reached through its code
                    // takes the same path as a call by name: arguments and
                    // body run through `closures::apply`, whose tail call
                    // keeps deep recursion (meta-eval) off the Rust stack.
                    Some(closure @ Value::Closure(_)) => {
                        return closures::apply(closure.clone(), arguments, environment, span);
                    }
                    _ => {}
                }
            }
            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate(argument, environment)?);
            }
            canon::invoke_semantic_ref(*sid, &values, environment, span)
                .map(EvalStep::Value)
        }
        Value::Builtin(builtin) => {
            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate(argument, environment)?);
            }
            (builtin.func)(&values, environment, span).map(EvalStep::Value)
        }
        Value::Macro(closure) => {
            closures::apply_macro(closure.clone(), arguments, environment, span)
        }
        _ => closures::apply(function, arguments, environment, span),
    }
}

/// A fixed-width binary token names a semantic identity only as a list head.
/// The same SID remains `Value::Sid` when it occurs as data or under
/// QUOTE, so a source file can carry bit data without making it executable.
fn binary_head_domain_identity(expression: &Expr) -> Option<CoreDomainIdentity> {
    let ExprKind::DomainIdentity(identity) = expression.kind else {
        return None;
    };
    identity.core_operation()
}

fn binary_head_sid(expression: &Expr) -> Option<CoreDomainIdentity> {
    let ExprKind::Sid(sid) = expression.kind else {
        return None;
    };
    Some(sid)
}

trait ExprKindExt {
    fn as_symbol(&self) -> Option<&str>;
}

impl ExprKindExt for ExprKind {
    fn as_symbol(&self) -> Option<&str> {
        match self {
            ExprKind::Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }
}

#[cfg(test)]
mod top_level_error_span_tests {
    use super::*;

    #[test]
    fn preserves_precise_span_and_recovers_missing_top_level_location() {
        let source = "; Початковий коментар\n(1)";
        let forms = parse(source).expect("джерельна форма");
        let form = &forms[0];
        assert_eq!(form.span.start, source.find("(1)").unwrap());

        let missing = LanguageError::new(ErrorKind::Type, "не можна викликати", Span::default());
        let repaired = top_level_error_span(missing, form);
        assert_eq!(repaired.span, form.span);
        assert_eq!(repaired.line_col(source), (2, 1));

        let precise = Span {
            start: form.span.start + 1,
            end: form.span.start + 2,
        };
        let already_located = LanguageError::new(ErrorKind::Type, "точний діапазон", precise);
        assert_eq!(top_level_error_span(already_located, form).span, precise);
    }
}

#[cfg(test)]
mod exact_domain_surface_value_position_tests {
    use super::*;

    #[test]
    fn bare_ukrainian_d3_surface_is_diagnosed_without_sid8() {
        let mut session = Session::default();
        let error = eval_program("атом?", &mut session)
            .expect_err("bare D3 function surface is not a value call");
        assert_eq!(error.kind, ErrorKind::InvalidForm);
        assert!(error.message.contains("D3 010"), "{}", error.message);
        assert!(!error.message.contains("SID"), "{}", error.message);
        assert!(!error.message.contains("00000010"), "{}", error.message);
    }
}

#[cfg(test)]
mod exact_domain_program_tests {
    use super::*;

    #[test]
    fn exact_d3_quote_keeps_the_current_d8_data_identity() {
        let mut session = Session::bare();
        let result = eval_exact_domain_program("(001 11110111)", &mut session)
            .expect("ратифікована D3 цитата виконується без історичного SID");
        assert!(matches!(
            result.value,
            Value::DomainIdentity(identity)
                if identity.width() == 8 && identity.packed_bits() == 0b11110111
        ));
    }

    #[test]
    fn exact_execution_rejects_historical_surface_before_any_side_effect() {
        let mut session = Session::bare();
        let error = eval_exact_domain_program("(def x (001 1))", &mut session)
            .expect_err("legacy-only def is not ratified D4 DEFINE");
        assert_eq!(error.kind, ErrorKind::InvalidForm);
        assert!(error.message.contains("historical-only"), "{}", error.message);
        assert_eq!(&"(def x (001 1))"[error.span.start..error.span.end], "def");
        assert!(session.environment.get("x").is_none());
    }

    #[test]
    fn exact_quote_preserves_historical_looking_symbols_as_inert_data() {
        let mut session = Session::bare();
        let result = eval_exact_domain_program("(001 (def x 11110111))", &mut session)
            .expect("D3 quoted syntax must not be mistaken for a call");
        assert!(matches!(result.value, Value::Pair(_, _)));
    }

    #[test]
    fn noncallable_current_d8_cannot_inherit_historical_quote_dispatch() {
        let mut session = Session::bare();
        let error = eval_exact_domain_program("(00000001 (001 1))", &mut session)
            .expect_err("D8:00000001 is not historical W8 QUOTE");
        assert!(matches!(error.kind, ErrorKind::Type | ErrorKind::MechanismUnavailable));
    }

    #[test]
    fn exact_source_rejects_implicit_decimal_literals() {
        let mut session = Session::bare();
        let error = eval_exact_domain_program("(001 42)", &mut session)
            .expect_err("числа потребують окремого допуску D24+");
        assert_eq!(error.kind, ErrorKind::Parse);
    }
}

#[cfg(test)]
mod domain_eval_cutover_necessary_forms {
    use super::*;

    fn arguments(source: &str) -> (Vec<Expr>, Span) {
        let parsed = parse(source).expect("допущена історична проєкція D4");
        let expression = &parsed[0];
        let ExprKind::List(parts) = &expression.kind else {
            panic!("очікуємо викличну форму");
        };
        (parts[1..].to_vec(), expression.span)
    }

    #[test]
    fn ukrainian_lambda_source_uses_exact_d4_mechanism() {
        let (args, span) = arguments("(функція (x) x)");
        let env = Environment::root();
        let via_source = dispatch_call(Some("функція"), None, None, &args, &env, span)
            .expect("джерельна проєкція D4:0010");
        let identity = necessary_forms::domain_identity_for_symbol("функція")
            .expect("ратифікована функція");
        assert!(matches!(via_source, EvalStep::Value(Value::Closure(_))));
        assert!(matches!(
            dispatch_domain_call(identity, &args, &env, span)
                .expect("точна D4:0010"),
            EvalStep::Value(Value::Closure(_))
        ));
        assert_eq!(identity.width(), 4);
    }

    #[test]
    fn ukrainian_define_source_uses_exact_d4_mechanism() {
        let (args, span) = arguments("(визначити перевірка ())");
        let source_env = Environment::root();
        let exact_env = Environment::root();
        let identity = necessary_forms::domain_identity_for_symbol("визначити")
            .expect("ратифікована D4:0011");
        dispatch_call(Some("визначити"), None, None, &args, &source_env, span)
            .expect("визначення через доменну таблицю");
        dispatch_domain_call(identity, &args, &exact_env, span)
            .expect("пряме визначення D4:0011");
        assert!(matches!(source_env.get("перевірка"), Some(Value::Nil)));
        assert!(matches!(exact_env.get("перевірка"), Some(Value::Nil)));
    }

    #[test]
    fn only_necessary_d4_heads_gain_this_route() {
        assert!(necessary_forms::domain_identity_for_symbol("функція").is_some());
        assert!(necessary_forms::domain_identity_for_symbol("визначити").is_some());
        // D3 ATOM, a foreign symbol, and an old bare byte must not inherit
        // the D4 necessary-form mechanism by payload or spelling.
        assert!(necessary_forms::domain_identity_for_symbol("атом?").is_none());
        assert!(necessary_forms::domain_identity_for_symbol("чужий-виклик").is_none());
        assert!(necessary_forms::domain_identity_for_symbol("00001000").is_none());
    }
}

#[cfg(test)]
mod domain_eval_cutover_d3_compatibility {
    use super::*;

    #[test]
    fn only_ratified_d3_projection_bridges_historical_quote_and_cond() {
        for (legacy, bits) in [
            (crate::CoreDomainIdentity::D3(crate::Bija3::from_word(crate::Bit3::new(0b001).expect("quote"))), 0b001u8),
            (crate::CoreDomainIdentity::D3(crate::Bija3::from_word(crate::Bit3::new(0b110).expect("cond"))), 0b110u8),
        ] {
            let identity = crate::semantic_registry::compatibility_d3_route_from_ratified_domains(legacy)
                .expect("історичний W8 має доказову проєкцію в D3");
            assert!(is_d3(identity, bits));
            assert_eq!(identity.width(), 3);
        }
        for unrelated in [crate::DomainIdentity::D10(crate::DomainD10::new(0b0001010111).expect("D10 funcall")), crate::sens!(11110111)] {
            assert!(
                crate::semantic_registry::compatibility_d3_route_from_ratified_domains(unrelated).is_none(),
                "невідома W8-комірка не може стати чинним D3/D8"
            );
        }
    }

    #[test]
    fn historical_quote_runs_identically_to_exact_d3_quote() {
        let mut legacy = Session::bare();
        let mut exact = Session::bare();
        let old_value = eval_program("(00000001 (a b))", &mut legacy)
            .expect("старе цитування має пройти засвідчений D3-маршрут");
        let new_value = eval_exact_domain_program("(001 (a b))", &mut exact)
            .expect("чинне точнодоменне цитування");
        assert_eq!(old_value.value, new_value.value);
    }

    #[test]
    fn legacy_cond_rejects_three_part_clause_by_current_d3_contract() {
        let mut session = Session::bare();
        let err = eval_program("(00000111 ((00000010 (00000001 ())) (00000001 yes) (00000001 no)))", &mut session)
            .expect_err("двопольний D3 COND забороняє історичну тричастинну клаузу");
        assert_eq!(err.kind, ErrorKind::InvalidForm);
        assert!(err.message.contains("D3:110"), "{}", err.message);
    }
}
