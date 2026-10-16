use sens::{
    eval_program, load_core_library, register_capability, Environment, ErrorKind, Exactness, Expr,
    LanguageError, Session, Span, Value,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_lisp_file(path: &str, session: &mut Session) {
    let path = repo_root().join(path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
    eval_program(&source, session)
        .unwrap_or_else(|error| panic!("{} must load as ordinary sens: {error}", path.display()));
}

fn spy_executor(
    _arguments: &[Expr],
    _environment: &Environment,
    _span: Span,
) -> Result<Value, LanguageError> {
    EXECUTOR_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(Value::Number(4242.0, Exactness::Exact))
}

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core must bootstrap before machine Type witness");
    load_lisp_file("lib/machine/layout/pair-x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-x86-64.lisp", &mut session);
    session
}

#[test]
fn semantic_car_entry_preserves_canonical_type_before_host() {
    let mut session = machine_session();
    register_capability("native-call-u64-raw", spy_executor);

    for (reference_source, machine_source) in [
        ("(car 5)", "(x86-call-semantic-car-u64 5)"),
        (
            "(car (quote ()))",
            "(x86-call-semantic-car-u64 (quote ()))",
        ),
    ] {
        let reference_error = eval_program(reference_source, &mut session)
            .expect_err("reference CAR must reject a non-pair");
        assert_eq!(
            reference_error.kind,
            ErrorKind::Type,
            "constitution requires canonical CAR non-pair outcome Type"
        );

        EXECUTOR_CALLS.store(0, Ordering::SeqCst);
        let machine_error = eval_program(machine_source, &mut session)
            .expect_err("machine semantic CAR entry must reject before native execution");
        assert_eq!(
            machine_error.kind, reference_error.kind,
            "machine semantic entry must preserve the canonical CAR error kind"
        );
        assert_eq!(
            EXECUTOR_CALLS.load(Ordering::SeqCst),
            0,
            "invalid CAR input must not produce machine forms that reach the host"
        );
    }

    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    let valid = eval_program(
        "(x86-call-semantic-car-u64 (cons 2 3))",
        &mut session,
    )
    .expect("valid bounded pair must proceed through the admitted machine gateway");
    assert_eq!(
        valid.value.to_string(),
        "4242",
        "valid semantic entry must return the executor result"
    );
    assert_eq!(
        EXECUTOR_CALLS.load(Ordering::SeqCst),
        1,
        "valid bounded pair must reach the host exactly once"
    );
}
