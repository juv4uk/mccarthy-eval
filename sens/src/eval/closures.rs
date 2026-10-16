//! `lambda` construction and applying closures/macros to arguments.
//! Pobudova `lambda` ta zastosuvannia zamykan/makrosiv do arhumentiv.
//! Bau von `lambda` und Anwendung von Closures/Makros auf Argumente.

use super::{canon, capabilities, evaluate, necessary_forms, special_forms::quoted, EvalStep};
use crate::canonical_reader::text7_binding_key;
use crate::semantic_registry;
use crate::{Closure, Environment, ErrorKind, Expr, ExprKind, LanguageError, CoreDomainIdentity, Span, Value};
use crate::CoreDomainIdentity;
use std::{
    collections::HashSet,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

/// Parses a lambda-list, which comes in three shapes shared across the Lisp
/// family (not one dialect's `&rest` keyword): `(a b)` — exactly two fixed
/// parameters, no rest; `(a b . rest)` — a dotted list, `rest` bound to
/// every argument past `a`/`b`; and a bare symbol `args` — zero fixed
/// parameters, every argument bound to `args`. The third shape reads as an
/// ordinary `ExprKind::Symbol`, the second as nested `ExprKind::Pair` (the
/// same dotted-pair reader support added earlier for data literals),
/// exactly the shapes `parser.rs` already produces — no new parser syntax.
/// Rozbyraie lambda-list, shcho maie try formy, spilni dlia rodyny Lisp (ne
/// kliuchove slovo `&rest` odnoho dialektu): `(a b)` — tochno dva fiksovani
/// parametry, bez rest; `(a b . rest)` — dotted-spysok, `rest` zviazuietsia
/// z usima arhumentamy ponad `a`/`b`; holyi symvol `args` — nul fiksovanykh
/// parametriv, kozhen arhument zviazuietsia z `args`. Tretia forma chytaietsia yak
/// zvychainyi `ExprKind::Symbol`, druha — yak vkladenyi `ExprKind::Pair` (ta
/// sama pidtrymka dotted-pair readera, dodana ranishe dlia literaliv danykh) —
/// same ti formy, yaki `parser.rs` uzhe y tak vyrobliaie, bez novoho syntaksysu.
fn parse_lambda_list(expr: &Expr) -> LambdaListResult {
    parse_lambda_list_inner(expr)
}

type LambdaList = (Vec<Rc<str>>, Option<Rc<str>>);
type LambdaListResult = Result<LambdaList, LanguageError>;

fn bindable_parameter_name(expr: &Expr) -> Result<Option<Rc<str>>, LanguageError> {
    let Some(name) = text7_binding_key(expr) else {
        return Ok(None);
    };
    canon::ensure_bindable(&name, expr.span)?;
    Ok(Some(name))
}

fn parse_lambda_list_inner(expr: &Expr) -> LambdaListResult {
    if let Some(name) = bindable_parameter_name(expr)? {
        return Ok((Vec::new(), Some(name)));
    }
    match &expr.kind {
        ExprKind::Symbol(name) => {
            canon::ensure_bindable(name, expr.span)?;
            Ok((Vec::new(), Some(name.clone())))
        }
        ExprKind::Sid(sid) => {
            canon::ensure_bindable_sid(*sid, expr.span)?;
            Ok((Vec::new(), Some(sid.to_string().into())))
        }
        ExprKind::DomainIdentity(identity) => {
            Err(canon::immutable_domain_binding_error(*identity, expr.span))
        }
        ExprKind::List(parameter_forms) => {
            let mut parameters = Vec::with_capacity(parameter_forms.len());
            let mut unique = HashSet::new();
            for parameter in parameter_forms.iter() {
                let name = match bindable_parameter_name(parameter)? {
                    Some(name) => name,
                    None => match &parameter.kind {
                    ExprKind::Symbol(name) => {
                        canon::ensure_bindable(name, parameter.span)?;
                        name.clone()
                    }
                    ExprKind::Sid(sid) => {
                        canon::ensure_bindable_sid(*sid, parameter.span)?;
                        sid.to_string().into()
                    }
                    ExprKind::DomainIdentity(identity) => {
                        return Err(canon::immutable_domain_binding_error(
                            *identity,
                            parameter.span,
                        ));
                    }
                    _ => {
                        return Err(LanguageError::new(
                            ErrorKind::InvalidForm,
                            "lambda parameter must be a symbol · parametr lambda maie buty symvolom · lambda-Parameter muss ein Symbol sein",
                            parameter.span,
                        ));
                    }
                },
                };
                if !unique.insert(name.clone()) {
                    return Err(LanguageError::new(
                        ErrorKind::InvalidForm,
                        format!("duplicate lambda parameter · povtornyi parametr lambda · doppelter lambda-Parameter: {name}"),
                        parameter.span,
                    ));
                }
                parameters.push(name);
            }
            Ok((parameters, None))
        }
        ExprKind::Pair(_, _) => {
            let mut parameters = Vec::new();
            let mut unique = HashSet::new();
            let mut current: &Expr = expr;
            let rest = loop {
                match &current.kind {
                    ExprKind::Pair(head, tail) => {
                        let name = match &head.kind {
                            ExprKind::Symbol(name) => {
                                canon::ensure_bindable(name, head.span)?;
                                name.clone()
                            }
                            ExprKind::Sid(sid) => {
                                canon::ensure_bindable_sid(*sid, head.span)?;
                                sid.to_string().into()
                            }
                            ExprKind::DomainIdentity(identity) => {
                                return Err(canon::immutable_domain_binding_error(
                                    *identity,
                                    head.span,
                                ));
                            }
                            _ => {
                                return Err(LanguageError::new(
                                    ErrorKind::InvalidForm,
                                    "lambda parameter must be a symbol · parametr lambda maie buty symvolom · lambda-Parameter muss ein Symbol sein",
                                    head.span,
                                ));
                            }
                        };
                        if !unique.insert(name.clone()) {
                            return Err(LanguageError::new(
                                ErrorKind::InvalidForm,
                                format!("duplicate lambda parameter · povtornyi parametr lambda · doppelter lambda-Parameter: {name}"),
                                head.span,
                            ));
                        }
                        parameters.push(name);
                        current = tail;
                    }
                    ExprKind::Symbol(name) => {
                        canon::ensure_bindable(name, current.span)?;
                        if !unique.insert(name.clone()) {
                            return Err(LanguageError::new(
                                ErrorKind::InvalidForm,
                                format!("duplicate lambda parameter · povtornyi parametr lambda · doppelter lambda-Parameter: {name}"),
                                current.span,
                            ));
                        }
                        break name.clone();
                    }
                    ExprKind::Sid(sid) => {
                        canon::ensure_bindable_sid(*sid, current.span)?;
                        break sid.to_string().into();
                    }
                    ExprKind::DomainIdentity(identity) => {
                        return Err(canon::immutable_domain_binding_error(
                            *identity,
                            current.span,
                        ));
                    }
                    _ => {
                        return Err(LanguageError::new(
                            ErrorKind::InvalidForm,
                            "rest parameter must be a symbol · rest-parametr maie buty symvolom · Rest-Parameter muss ein Symbol sein",
                            current.span,
                        ));
                    }
                }
            };
            Ok((parameters, Some(rest)))
        }
        _ => Err(LanguageError::new(
            ErrorKind::InvalidForm,
            "lambda parameters must be a list, dotted list, or symbol · parametry lambda maiut buty spyskom, dotted-spyskom abo symvolom · lambda-Parameter müssen eine Liste, Dotted-Liste oder ein Symbol sein",
            expr.span,
        )),
    }
}

pub(super) fn create_lambda(
    arguments: &[Expr],
    environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    if arguments.len() < 2 {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            "lambda expects parameters and a body · lambda ochikuie parametry y tilo · lambda erwartet Parameter und einen Rumpf",
            span,
        ));
    }
    let (parameters, rest) = parse_lambda_list(&arguments[0])?;
    let slot_names: Rc<[Rc<str>]> = parameters.iter().chain(rest.iter()).cloned().collect();
    let body: Rc<[Expr]> = arguments[1..].into();
    let pure = body.iter().all(|expression| is_pure(expression, &slot_names, environment));
    let resolved = resolve_body(&body, &slot_names, pure, environment);
    Ok(Value::Closure(Rc::new(Closure {
        parameters,
        rest,
        body,
        environment: environment.clone(),
        slot_names,
        pure,
        resolved,
    })))
}

// --- параметри за номером слота ---------------------------------------------
//
// Розв'язувач замінює посилання на параметр замикання вузлом
// `ExprKind::Local { depth, index }` там, де обчислення цього імені гарантовано
// знайшло б саме цей слот. Семантика не змінюється: кадр виклику зберігає й
// імена параметрів, тож `eval`, знімки оточення й нерозв'язані посилання
// працюють за іменем, як раніше.
//
// Посилання не розв'язуються в даних (`quote`, очікуваний результат `cond`),
// в аргументах макросів і можливостей хоста (вони бачать синтаксис), у
// вкладених `lambda` (їх розв'яже власне створення), у голові виклику (вона
// лишається іменем для диспетчера). Крізь кадр, тіло якого може додати нове
// ім'я (`def`, `eval`, макрос, можливість хоста, вбудована функція), глибше не
// дивимося. Новий макрос чи можливість хоста піднімає епоху — розв'язані
// раніше тіла відкидаються на користь оригінальних.

static RESOLUTION_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Новий макрос або можливість хоста можуть змінити, які форми бачать синтаксис.
pub(crate) fn bump_resolution_epoch() {
    RESOLUTION_EPOCH.fetch_add(1, Ordering::Relaxed);
}

fn resolution_epoch() -> u64 {
    RESOLUTION_EPOCH.load(Ordering::Relaxed)
}

/// Тіло для виконання: розв'язане, якщо його епоха ще чинна.
fn body_for_call(closure: &Closure) -> &Rc<[Expr]> {
    match &closure.resolved {
        Some((body, epoch)) if *epoch == resolution_epoch() => body,
        _ => &closure.body,
    }
}

/// Кадр виклику з уже обчисленими значеннями параметрів.
fn call_frame(closure: &Closure, slots: Vec<Value>) -> Environment {
    closure
        .environment
        .child_with_slots(closure.slot_names.clone(), slots, closure.pure)
}

/// Як диспетчер виклику бачить голову форми.
enum Head {
    Quote,
    Lambda,
    Define,
    Cond,
    /// Макрос, можливість хоста, вбудована функція чи `eval`: аргументи — не
    /// звичайні обчислювані вирази або виклик може додати ім'я в кадр.
    Opaque,
    /// Звичайний виклик: аргументи обчислюються як вирази.
    Call,
}

fn domain_head(identity: CoreDomainIdentity, environment: &Environment) -> Head {
    if let Some(necessary) = necessary_forms::identity_for_domain_identity(identity) {
        return match necessary {
            necessary_forms::NecessaryFormIdentity::Lambda => Head::Lambda,
            necessary_forms::NecessaryFormIdentity::Define => Head::Define,
        };
    }

    let routed = match identity {
        CoreDomainIdentity::D3(word) => match word.word().packed_bits() {
            0b001 => Head::Quote,
            0b110 => Head::Cond,
            _ => Head::Call,
        },
        // D4:0001 is the ratified EVAL word. It can execute code and must
        // never be misclassified as an ordinary pure value-call.
        CoreDomainIdentity::D4(word) if word.word().packed_bits() == 0b0001 => Head::Opaque,
        CoreDomainIdentity::D5(word) if word.word().packed_bits() == 0b00100 => Head::Opaque,
        _ => Head::Call,
    };
    if matches!(routed, Head::Call)
        && matches!(environment.domain_code_slot(identity), Some(Value::Macro(_)))
    {
        Head::Opaque
    } else {
        routed
    }
}

fn sid_head(sid: CoreDomainIdentity, environment: &Environment) -> Head {
    // Historical W8 EVAL remains a compatibility mechanism only. Obtain
    // its slot from the Lisp-owned generated registry, not a Rust byte law.
    if semantic_registry::admitted_semantic_id_for_surface("eval") == Some(sid) {
        return Head::Opaque;
    }
    match necessary_forms::identity_for_semantic_id(sid) {
        Some(necessary_forms::NecessaryFormIdentity::Lambda) => return Head::Lambda,
        Some(necessary_forms::NecessaryFormIdentity::Define) => return Head::Define,
        _ => {}
    }
    // Compatibility source heads inherit only their ratified D3 law.
    // No old eight-bit payload is itself a QUOTE/COND semantic authority.
    if let Some(CoreDomainIdentity::D3(word)) =
        semantic_registry::compatibility_d3_route_from_ratified_domains(sid)
    {
        match word.word().packed_bits() {
            0b001 => return Head::Quote,
            0b110 => return Head::Cond,
            _ => {}
        }
    }
    if !canon::has_primitive(sid) && matches!(environment.code_slot(sid), Some(Value::Macro(_))) {
        return Head::Opaque;
    }
    Head::Call
}

fn classify_head(head: &Expr, own: &[Rc<str>], environment: &Environment) -> Head {
    match &head.kind {
        ExprKind::DomainIdentity(identity) => identity
            .core_operation()
            .map(|core| domain_head(core, environment))
            .unwrap_or(Head::Opaque),
        ExprKind::Sid(sid) => sid_head(*sid, environment),
        ExprKind::Symbol(name) => {
            // Ratified exact-domain surfaces must reach the same head policy as
            // DomainCall. In particular, Ukrainian D3 QUOTE/COND are not
            // reclassified through their historical SID8 compatibility bytes.
            if let Some(identity) = super::lower::immutable_surface_domain_identity(name) {
                return domain_head(identity, environment);
            }
            if let Some(sid) = canon::routed_sid_for_surface(name) {
                return sid_head(sid, environment);
            }
            match necessary_forms::identity_for_symbol(name) {
                Some(necessary_forms::NecessaryFormIdentity::Lambda) => return Head::Lambda,
                Some(necessary_forms::NecessaryFormIdentity::Define) => return Head::Define,
                _ => {}
            }
            if capabilities::capability_installed(name) {
                return Head::Opaque;
            }
            if own.iter().any(|parameter| **parameter == **name) {
                return Head::Call;
            }
            match environment.get(name) {
                Some(Value::Macro(_) | Value::Builtin(_)) => Head::Opaque,
                Some(Value::DomainIdentity(identity)) => identity
                    .core_operation()
                    .map(|core| domain_head(core, environment))
                    .unwrap_or(Head::Opaque),
                Some(Value::Sid(sid)) => sid_head(sid, environment),
                _ => Head::Call,
            }
        }
        _ => Head::Call,
    }
}

/// Чи тіло не може додати нове ім'я в кадр виклику.
fn is_pure(expression: &Expr, own: &[Rc<str>], environment: &Environment) -> bool {
    let (head, arguments): (Head, &[Expr]) = match &expression.kind {
        ExprKind::List(items) if !items.is_empty() => {
            (classify_head(&items[0], own, environment), &items[..])
        }
        ExprKind::Call(sid, arguments) => (sid_head(*sid, environment), &arguments[..]),
        ExprKind::DomainCall(identity, arguments) => {
            (domain_head(*identity, environment), &arguments[..])
        },
        _ => return true,
    };
    match head {
        Head::Quote | Head::Lambda => true,
        Head::Define | Head::Opaque => false,
        Head::Cond | Head::Call => arguments.iter().all(|part| match &part.kind {
            ExprKind::List(clause) if matches!(head, Head::Cond) => {
                clause.iter().all(|item| is_pure(item, own, environment))
            }
            _ => is_pure(part, own, environment),
        }),
    }
}

fn resolve_body(
    body: &Rc<[Expr]>,
    own: &Rc<[Rc<str>]>,
    pure: bool,
    environment: &Environment,
) -> Option<(Rc<[Expr]>, u64)> {
    let epoch = resolution_epoch();
    let mut scopes = vec![own.clone()];
    if pure {
        scopes.extend(environment.lexical_slot_scopes());
    }
    let mut changed = false;
    let resolved: Rc<[Expr]> = body
        .iter()
        .map(|expression| resolve(expression, &scopes, environment, &mut changed))
        .collect();
    changed.then_some((resolved, epoch))
}

fn local_for(name: &Rc<str>, scopes: &[Rc<[Rc<str>]>]) -> Option<ExprKind> {
    // Exact-domain immutable heads are never captured as lambda locals.
    // Keep the historical byte route only as a fallback for unmigrated heads.
    if canon::is_reserved_surface(name)
        || necessary_forms::identity_for_symbol(name).is_some()
    {
        return None;
    }
    scopes.iter().enumerate().find_map(|(depth, names)| {
        names.iter().position(|slot| **slot == **name).map(|index| ExprKind::Local {
            depth: depth as u32,
            index: index as u32,
        })
    })
}

fn resolve(
    expression: &Expr,
    scopes: &[Rc<[Rc<str>]>],
    environment: &Environment,
    changed: &mut bool,
) -> Expr {
    let resolve_all = |items: &[Expr], changed: &mut bool| -> Rc<[Expr]> {
        items.iter().map(|item| resolve(item, scopes, environment, changed)).collect()
    };
    let kind = match &expression.kind {
        ExprKind::Symbol(name) => match local_for(name, scopes) {
            Some(local) => {
                *changed = true;
                local
            }
            None => return expression.clone(),
        },
        ExprKind::List(items) if !items.is_empty() => {
            match classify_head(&items[0], &scopes[0], environment) {
                Head::Quote | Head::Lambda | Head::Define | Head::Opaque => return expression.clone(),
                Head::Cond => {
                    let mut out = Vec::with_capacity(items.len());
                    out.push(items[0].clone());
                    for clause in &items[1..] {
                        out.push(resolve_cond_clause(clause, scopes, environment, changed));
                    }
                    ExprKind::List(out.into())
                }
                Head::Call => {
                    // Голова лишається як є: диспетчер вирішує за нею.
                    let head = match &items[0].kind {
                        ExprKind::Symbol(_) | ExprKind::Sid(_) | ExprKind::DomainIdentity(_) => {
                            items[0].clone()
                        },
                        _ => resolve(&items[0], scopes, environment, changed),
                    };
                    let mut out = Vec::with_capacity(items.len());
                    out.push(head);
                    out.extend(items[1..].iter().map(|item| resolve(item, scopes, environment, changed)));
                    ExprKind::List(out.into())
                }
            }
        }
        ExprKind::Call(sid, arguments) => match sid_head(*sid, environment) {
            Head::Call => ExprKind::Call(*sid, resolve_all(arguments, changed)),
            Head::Cond => ExprKind::Call(
                *sid,
                arguments
                    .iter()
                    .map(|clause| resolve_cond_clause(clause, scopes, environment, changed))
                    .collect(),
            ),
            _ => return expression.clone(),
        },
        ExprKind::DomainCall(identity, arguments) => match domain_head(*identity, environment) {
            Head::Call => ExprKind::DomainCall(*identity, resolve_all(arguments, changed)),
            Head::Cond => ExprKind::DomainCall(
                *identity,
                arguments
                    .iter()
                    .map(|clause| resolve_cond_clause(clause, scopes, environment, changed))
                    .collect(),
            ),
            _ => return expression.clone(),
        },
        _ => return expression.clone(),
    };
    Expr {
        kind,
        span: expression.span,
    }
}

/// Клауза `cond`: `(запит очікуване вираз)` — очікуване є даними;
/// `(перевірка вираз)` — обидва вирази. Інша форма лишається як є.
fn resolve_cond_clause(
    clause: &Expr,
    scopes: &[Rc<[Rc<str>]>],
    environment: &Environment,
    changed: &mut bool,
) -> Expr {
    let ExprKind::List(parts) = &clause.kind else {
        return clause.clone();
    };
    let parts: Rc<[Expr]> = match parts.len() {
        3 => [
            resolve(&parts[0], scopes, environment, changed),
            parts[1].clone(),
            resolve(&parts[2], scopes, environment, changed),
        ]
        .into(),
        2 => [
            resolve(&parts[0], scopes, environment, changed),
            resolve(&parts[1], scopes, environment, changed),
        ]
        .into(),
        _ => return clause.clone(),
    };
    Expr {
        kind: ExprKind::List(parts),
        span: clause.span,
    }
}

/// Shared by `apply`/`apply_macro`: exact arity when there's no rest
/// parameter, "at least this many" when there is.
/// Spilne dlia `apply`/`apply_macro`: tochna arnist, yakshcho nema rest-
/// parametra, "shchonaimenshe stilky" — yakshcho ye.
fn check_arity(
    label: &str,
    fixed: usize,
    has_rest: bool,
    received: usize,
    span: Span,
) -> Result<(), LanguageError> {
    let arity_ok = if has_rest {
        received >= fixed
    } else {
        received == fixed
    };
    if arity_ok {
        return Ok(());
    }
    let expected = if has_rest {
        format!("at least / shchonaimenshe / mindestens {fixed}")
    } else {
        fixed.to_string()
    };
    Err(LanguageError::new(
        ErrorKind::Arity,
        format!(
            "{label}: expected / ochikuvalosia / erwartet {expected}; received / otrymano / erhalten {received}"
        ),
        span,
    ))
}

pub(super) fn apply(
    function: Value,
    arguments: &[Expr],
    calling_environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    match function {
        Value::DomainIdentity(identity) => {
            let Some(core_identity) = identity.core_operation() else {
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
            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate(argument, calling_environment)?);
            }
            canon::invoke_domain_identity(core_identity, &values, calling_environment, span)
                .map(EvalStep::Value)
        }
        Value::Sid(sid) => {
            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate(argument, calling_environment)?);
            }
            canon::invoke_semantic_ref(sid, &values, calling_environment, span).map(EvalStep::Value)
        }
        Value::Closure(ref closure) => {
            check_arity(
                "lambda",
                closure.parameters.len(),
                closure.rest.is_some(),
                arguments.len(),
                span,
            )?;

            // Arguments belong to the caller; parameters belong to the captured lexical frame.
            // Arhumenty nalezhat vyklyku, a parametry — zakhoplenomu leksychnomu freimu.
            // Argumente gehören zum Aufrufer, Parameter zum erfassten lexikalischen Frame.
            let mut slots = Vec::with_capacity(closure.slot_names.len());
            for argument in &arguments[..closure.parameters.len()] {
                slots.push(evaluate(argument, calling_environment)?);
            }
            if closure.rest.is_some() {
                let mut rest_values = Vec::with_capacity(arguments.len() - closure.parameters.len());
                for argument in &arguments[closure.parameters.len()..] {
                    rest_values.push(evaluate(argument, calling_environment)?);
                }
                slots.push(Value::list(rest_values));
            }
            let local_environment = call_frame(closure, slots);
            let last = last_body_expression(body_for_call(closure), &local_environment, span)?;
            // Tail positions become data for the evaluator loop instead of recursive Rust calls.
            // Khvostovi pozytsii staiut danymy dlia tsyklu evaluator, a ne rekursyvnymy vyklykamy Rust.
            // Tail-Positionen werden zu Daten für den Evaluator-Schleife statt zu rekursiven Rust-Aufrufen.
            Ok(EvalStep::TailCall {
                expression: last.clone(),
                environment: local_environment,
            })
        }
        _ => Err(LanguageError::new(
            ErrorKind::Type,
            format!(
                "expression is not callable · vyraz ne mozhna vyklykaty · Ausdruck ist nicht aufrufbar (носій: {}{})",
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
                match function {
                    Value::Number(_, _) | Value::Rational(_) | Value::BinaryNumber(_) => {
                        format!("; значення: {}", function.to_canonical_wire_string())
                    }
                    _ => String::new(),
                },
            ),
            span,
        )),
    }
}

/// Apply an ordinary closure to values that are already evaluated. Bulk
/// primitives use this path so they do not fabricate source expressions or
/// accidentally evaluate an element as code.
pub(super) fn apply_values(
    closure: Rc<Closure>,
    arguments: &[Value],
    span: Span,
) -> Result<Value, LanguageError> {
    check_arity(
        "lambda",
        closure.parameters.len(),
        closure.rest.is_some(),
        arguments.len(),
        span,
    )?;
    let mut slots: Vec<Value> = arguments[..closure.parameters.len()].to_vec();
    if closure.rest.is_some() {
        slots.push(Value::list(arguments[closure.parameters.len()..].iter().cloned()));
    }
    let local_environment = call_frame(&closure, slots);
    let mut result = Value::Nil;
    for expression in body_for_call(&closure).iter() {
        result = evaluate(expression, &local_environment)?;
    }
    Ok(result)
}

/// Runs every body expression except the last for its side effects, then returns
/// the last one for the caller to evaluate in tail position. `create_lambda` always
/// builds a non-empty body, but this returns a `LanguageError` instead of panicking
/// so a future invariant change degrades gracefully rather than crashing the process.
/// Vykonuie vsi vyrazy tila, krim ostannoho, zarady pobichnykh efektiv, i povertaie
/// ostannii, shchob vyklyk obchyslyv yoho v khvostovii pozytsii. `create_lambda` zavzhdy
/// buduie neporozhnie tilo, ale tut povertaietsia `LanguageError`, a ne panika, shchob
/// maibutnia zmina invariantu dehraduvala plavno, a ne avariino zavershuvala protses.
/// Führt alle Rumpf-Ausdrücke außer dem letzten wegen ihrer Seiteneffekte aus und
/// gibt den letzten für die Auswertung in Tail-Position zurück. `create_lambda` baut
/// stets einen nicht leeren Rumpf, dennoch wird hier ein `LanguageError` statt eines
/// Panics zurückgegeben, damit eine künftige Invariantenänderung sanft statt abstürzend degradiert.
fn last_body_expression<'a>(
    body: &'a [Expr],
    environment: &Environment,
    span: Span,
) -> Result<&'a Expr, LanguageError> {
    let Some((last, leading)) = body.split_last() else {
        return Err(LanguageError::new(
            ErrorKind::InvalidForm,
            "lambda body must not be empty · tilo lambda ne mozhe buty porozhnim · lambda-Rumpf darf nicht leer sein",
            span,
        ));
    };
    for expression in leading {
        evaluate(expression, environment)?;
    }
    Ok(last)
}

pub(super) fn apply_macro(
    closure: Rc<Closure>,
    arguments: &[Expr],
    calling_environment: &Environment,
    span: Span,
) -> Result<EvalStep, LanguageError> {
    check_arity(
        "defmacro",
        closure.parameters.len(),
        closure.rest.is_some(),
        arguments.len(),
        span,
    )?;

    let mut slots = Vec::with_capacity(closure.slot_names.len());
    for argument in &arguments[..closure.parameters.len()] {
        slots.push(quoted(argument)?); // Do NOT evaluate arguments
    }
    if closure.rest.is_some() {
        let mut rest_values: Vec<Value> = Vec::with_capacity(arguments.len());
        for argument in &arguments[closure.parameters.len()..] {
            rest_values.push(quoted(argument)?); // Do NOT evaluate arguments
        }
        slots.push(Value::list(rest_values));
    }
    let local_environment = call_frame(&closure, slots);

    let last = last_body_expression(body_for_call(&closure), &local_environment, span)?;

    let expanded_value = evaluate(last, &local_environment)?;
    let expanded_expr = value_to_expr(expanded_value, span)?;

    Ok(EvalStep::TailCall {
        expression: expanded_expr,
        environment: calling_environment.clone(),
    })
}

// `pub(super)`, not private: `special_forms::evaluate_eval` reuses this same
// data->code conversion for `eval`, rather than duplicating the cons-cell
// walk that macro expansion already needed.
// `pub(super)`, ne pryvatna: `special_forms::evaluate_eval` perevykorystovuie
// tse same peretvorennia dani->kod dlia `eval`, zamist dubliuvannia obkhodu
// cons-komirok, yakyi uzhe buv potriben dlia rozhortannia makrosiv.
// `pub(super)`, nicht privat: `special_forms::evaluate_eval` nutzt dieselbe
// Daten->Code-Umwandlung für `eval` wieder, statt den Cons-Zellen-Durchlauf
// zu duplizieren, den die Makro-Expansion bereits brauchte.
pub(super) fn value_to_expr(value: Value, span: Span) -> Result<Expr, LanguageError> {
    fn go(value: &Value, span: Span, depth: u32) -> Result<Expr, LanguageError> {
        if depth > crate::syntax::MAX_STRUCTURE_DEPTH {
            return Err(LanguageError::new(
                ErrorKind::Parse,
                "structure exceeds reader limit · struktura perevyshchuie mezhu chytacha · Struktur überschreitet das Reader-Limit",
                span,
            ));
        }
        go_inner(value, span, depth)
    }

    fn go_inner(value: &Value, span: Span, depth: u32) -> Result<Expr, LanguageError> {
        let kind = match &value {
        Value::Nil => ExprKind::List(Rc::new([])),
        Value::Bool(true) => ExprKind::Symbol("t".into()),
        Value::Bool(false) => ExprKind::List(Rc::new([])),
        Value::Number(number, exactness) => ExprKind::Number(*number, *exactness),
        Value::Rational(rational) => ExprKind::Rational(rational.clone()),
        Value::BinaryNumber(number) => ExprKind::BinaryNumber(number.clone()),
        Value::NumericComposition(_) => return Err(LanguageError::new(ErrorKind::Type,
            "domain-qualified NEG/RATIO has no admitted source expression or macro code projection", span)),
        Value::Sid(sid) => ExprKind::Sid(*sid),
        Value::DomainIdentity(identity) => ExprKind::DomainIdentity(*identity),
        Value::NumericBuffer(buffer) => ExprKind::NumericBuffer(buffer.clone()),
        Value::String(val) => ExprKind::String(val.clone()),
        // A legacy host builtin is callable but not syntax either.
        Value::Builtin(builtin) => {
            return Err(LanguageError::new(
                ErrorKind::Type,
                format!("a builtin ({}) is not executable code · v budovanyi funktsii ne ye vykonavym kodom", builtin.name),
                span,
            ));
        }
        Value::Symbol(symbol) => ExprKind::Symbol(symbol.clone()),
        Value::Vector(_) => {
            return Err(LanguageError::new(
                crate::ErrorKind::Type,
                "a vector is not executable code",
                span,
            ));
        }
        Value::Pair(_, _) => {
            let mut items = Vec::new();
            let mut current = value.clone();
            loop {
                match &current {
                    Value::Pair(h, t) => {
                        items.push(go(h, span, depth + 1)?);
                        current = (**t).clone();
                    }
                    Value::Nil => break,
                    _ => {
                        return Err(LanguageError::new(
                            ErrorKind::InvalidForm,
                            "macros must return proper lists · makrosy povynni povertaty pravylni spysky · Makros müssen korrekte Listen zurückgeben",
                            span,
                        ))
                    }
                }
            }
            ExprKind::List(items.into())
        }
        Value::Closure(_) | Value::Macro(_) => {
            return Err(LanguageError::new(
                ErrorKind::InvalidForm,
                "macros cannot return closures or macros · makrosy ne mozhut povertaty zamykannia abo makrosy · Makros dürfen keine Closures oder Makros zurückgeben",
                span,
            ))
        }
        // No source syntax represents a live connection/listener — same
        // capability-boundary reasoning as Closure/Macro above, for the
        // same reason: quoting/macro-expanding one would need to fabricate
        // code that could recreate a specific open TCP resource, which
        // isn't a thing this language can express.
        // Zhoden syntaksys ne predstavliaie zhyve ziednannia/listener — ta sama
        // lohika mezhi mozhlyvostei, shcho y dlia Closure/Macro vyshche, z tiiei
        // samoi prychyny: quote/rozhortannia makrosa na tsomu znachenni
        // oznachalo b vyhadaty kod, shcho vidtvoryv by konkretnyi vidkrytyi
        // TCP-resurs, — tse ne te, shcho tsia mova mozhe vyrazyty.
        Value::HostHandle { .. } | Value::TcpConnection(_) | Value::TcpListener(_) => {
            return Err(LanguageError::new(
                ErrorKind::InvalidForm,
                "macros cannot return TCP connections or listeners · makrosy ne mozhut povertaty TCP-ziednannia chy listener · Makros dürfen keine TCP-Verbindungen oder Listener zurückgeben",
                span,
            ))
        }
        // Text7 is canonical data, not executable code: no source syntax
        // re-reads it yet (reader admission is a separate slice), so a macro
        // cannot return one as code. Named refusal, never silent bits.
        Value::Text7(_) => {
            return Err(LanguageError::new(
                ErrorKind::Type,
                "a Text7 value is not executable code · znachennia Text7 ne ye vykonavym kodom",
                span,
            ))
        }
    };
        Ok(Expr { kind, span })
    }

    go(&value, span, 0)
}

#[cfg(test)]
mod exact_d4_eval_head_migration_tests {
    use super::*;

    fn d4(bits: u8) -> CoreDomainIdentity {
        CoreDomainIdentity::D4(crate::CoreD4::from_word(
            crate::Bit4::new(bits).expect("точна ширина D4"),
        ))
    }

    #[test]
    fn exact_d4_eval_is_opaque_and_not_a_plain_pure_call() {
        let env = Environment::root();
        assert!(matches!(domain_head(d4(0b0001), &env), Head::Opaque));
        // Equal numeric payload in a different domain never becomes D4 EVAL.
        let d3 = CoreDomainIdentity::D3(crate::Bija3::from_word(
            crate::Bit3::new(0b001).unwrap(),
        ));
        assert!(matches!(domain_head(d3, &env), Head::Quote));
        // D4:0010 and D4:0011 are ratified syntax mechanisms, not
        // ordinary calls. The historical assertion expected the opposite.
        assert!(matches!(domain_head(d4(0b0010), &env), Head::Lambda));
        assert!(matches!(domain_head(d4(0b0011), &env), Head::Define));
    }

    #[test]
    fn historical_eval_uses_registry_slot_but_other_w8_is_not_eval() {
        let env = Environment::root();
        let historical = semantic_registry::admitted_semantic_id_for_surface("eval")
            .expect("історична таблиця має окремий сумісний слот EVAL");
        assert_eq!(historical.packed_byte(), 0b01001101);
        assert!(matches!(sid_head(historical, &env), Head::Opaque));
        assert!(!matches!(sid_head(crate::DomainIdentity::D10(crate::DomainD10::new(0b0001010111).expect("D10 funcall")), &env), Head::Opaque));
    }

    #[test]
    fn historical_quote_and_cond_require_admitted_d3_laws() {
        let env = Environment::root();
        let historical_quote = semantic_registry::admitted_semantic_id_for_surface("quote")
            .expect("історична QUOTE мітка");
        let historical_cond = semantic_registry::admitted_semantic_id_for_surface("cond")
            .expect("історична COND мітка");
        assert!(matches!(sid_head(historical_quote, &env), Head::Quote));
        assert!(matches!(sid_head(historical_cond, &env), Head::Cond));
    }
}

#[cfg(test)]
mod exact_domain_head_classification_tests {
    use super::*;

    #[test]
    fn ratified_surfaces_classify_like_exact_domain_heads() {
        let environment = Environment::root();
        let own: &[Rc<str>] = &[];
        for (source, expected) in [
            ("як-є", "quote"),
            ("за-умовою", "cond"),
            ("атом?", "call"),
            ("функція", "lambda"),
            ("визначити", "define"),
        ] {
            let parsed = crate::parse(source).expect("canonical surface parses");
            let actual = classify_head(&parsed[0], own, &environment);
            let valid = match (actual, expected) {
                (Head::Quote, "quote")
                | (Head::Cond, "cond")
                | (Head::Call, "call")
                | (Head::Lambda, "lambda")
                | (Head::Define, "define") => true,
                _ => false,
            };
            assert!(valid, "surface {source} must use ratified domain head policy");
        }
    }
}
