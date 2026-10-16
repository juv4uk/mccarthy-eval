use crate::value::Builtin;
use crate::{Environment, ErrorKind, LanguageError, Span, Value};
use std::rc::Rc;

/// Install the narrow bootstrap mechanism that turns an evaluated closure
/// into a macro value. `defmacro` itself remains language-owned in
/// `lib/macro.lisp`; this binding only supplies Closure -> Macro materialization.
pub(crate) fn install(environment: &Environment) {
    environment.define(
        "make-macro",
        Value::Builtin(Rc::new(Builtin {
            name: "make-macro",
            func: Rc::new(make_macro_values),
        })),
    );
}

fn make_macro_values(
    arguments: &[Value],
    _environment: &Environment,
    span: Span,
) -> Result<Value, LanguageError> {
    if arguments.len() != 1 {
        return Err(LanguageError::new(
            ErrorKind::Arity,
            "make-macro expects exactly 1 argument(s)",
            span,
        ));
    }

    match &arguments[0] {
        Value::Closure(closure) => {
            // Новий макрос: розв'язані раніше тіла могли бачити цю назву як виклик.
            super::closures::bump_resolution_epoch();
            Ok(Value::Macro(closure.clone()))
        }
        _ => Err(LanguageError::new(
            ErrorKind::Type,
            "make-macro expects a closure · make-macro ochikuie zamykannia · make-macro erwartet eine Closure",
            span,
        )),
    }
}
