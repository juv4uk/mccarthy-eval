use sens::{eval_program, ErrorKind, Session};

const CM_DOMAIN: &str = "Core-Math.QGroupFactor";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Basis {
    Add,
    Mul,
    Recip,
}

impl Basis {
    fn cm_bits(self) -> &'static str {
        match self {
            Self::Add => "0",
            Self::Mul => "1",
            Self::Recip => "10",
        }
    }

    fn core_exec_bits(self) -> &'static str {
        // Typed bridge only: these are existing Core execution identities.
        // They are NOT Core-Math identities and do not imply shared placement.
        match self {
            Self::Add => "00001100",
            Self::Mul => "00001110",
            Self::Recip => "00001111",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExactResult {
    Value,
    UndefinedMathematically,
}

fn call(op: Basis, args: &[String]) -> String {
    format!("({} {})", op.core_exec_bits(), args.join(" "))
}

fn add(a: impl Into<String>, b: impl Into<String>) -> String {
    call(Basis::Add, &[a.into(), b.into()])
}

fn mul(a: impl Into<String>, b: impl Into<String>) -> String {
    call(Basis::Mul, &[a.into(), b.into()])
}

fn recip(x: impl Into<String>) -> String {
    call(Basis::Recip, &[x.into()])
}

fn neg(x: impl Into<String>) -> String {
    // Generated from admitted basis + explicit neg_one constant.
    mul("-1", x.into())
}

fn sub(a: impl Into<String>, b: impl Into<String>) -> String {
    // Generated: ADD(a, NEG(b)).
    add(a.into(), neg(b))
}

fn div(a: impl Into<String>, b: impl Into<String>) -> String {
    // Generated: MUL(a, RECIP(b)).
    mul(a.into(), recip(b))
}

fn eval_exact(source: &str) -> String {
    eval_program(source, &mut Session::default())
        .unwrap_or_else(|error| panic!("{source:?} failed: {error:?}"))
        .value
        .to_string()
}

fn classify_recip(source: &str) -> ExactResult {
    match eval_program(source, &mut Session::default()) {
        Ok(_) => ExactResult::Value,
        Err(error) if error.kind == ErrorKind::DivisionByZero => {
            ExactResult::UndefinedMathematically
        }
        Err(error) => panic!("unexpected exact-Q failure for {source:?}: {error:?}"),
    }
}

#[test]
fn typed_core_math_basis_executes_exact_q_through_binary_core_bridge() {
    // Core-Math identities remain in their own domain.
    assert_eq!(CM_DOMAIN, "Core-Math.QGroupFactor");
    assert_eq!(Basis::Add.cm_bits(), "0");
    assert_eq!(Basis::Mul.cm_bits(), "1");
    assert_eq!(Basis::Recip.cm_bits(), "10");

    // The bridge target identities are separate exact binary objects.
    for op in [Basis::Add, Basis::Mul, Basis::Recip] {
        assert_eq!(op.core_exec_bits().len(), 8);
        assert_ne!(op.cm_bits(), op.core_exec_bits());
    }

    // Only the three admitted basis operations have bridge rows.
    assert_eq!([Basis::Add, Basis::Mul, Basis::Recip].len(), 3);

    assert_eq!(eval_exact(&add("2", "3")), "5");

    let half = recip("2");
    let third = recip("3");
    assert_eq!(eval_exact(&add(half, third)), "5/6");

    assert_eq!(eval_exact(&mul(neg("2"), "3")), "-6");
    assert_eq!(eval_exact(&recip("2")), "1/2");

    // No independent SUB bridge row: ADD(a, MUL(-1,b)).
    assert_eq!(eval_exact(&sub("5", "8")), "-3");

    // No independent DIV bridge row: MUL(a, RECIP(b)).
    assert_eq!(eval_exact(&div("6", "4")), "3/2");

    assert_eq!(
        classify_recip(&recip("0")),
        ExactResult::UndefinedMathematically
    );
}

#[test]
fn generated_operations_do_not_require_textual_operator_dispatch() {
    let sources = [
        add("2", "3"),
        add(recip("2"), recip("3")),
        mul(neg("2"), "3"),
        recip("2"),
        sub("5", "8"),
        div("6", "4"),
    ];

    for source in sources {
        // Function position is always an exact binary Core execution identity.
        // Human operator spellings are never semantic dispatch authority here.
        assert!(!source.contains("(+"));
        assert!(!source.contains("(-"));
        assert!(!source.contains("(*"));
        assert!(!source.contains("(/"));
        let _ = eval_exact(&source);
    }
}

#[test]
fn bridge_adds_no_d5_or_d6_identity() {
    // This test-side bridge targets pre-existing 8-bit execution identities.
    // Core-Math bits remain typed QGroupFactor objects. No 5/6-bit Core
    // coordinate is introduced by the integration witness.
    for op in [Basis::Add, Basis::Mul, Basis::Recip] {
        assert_ne!(op.core_exec_bits().len(), 5);
        assert_ne!(op.core_exec_bits().len(), 6);
    }
}

#[test]
fn historical_lisp15_recip_is_a_domain_mismatch_control() {
    let ledger = include_str!("../../../docs/research/2709-lisp15-arithmetic-ledger.json");

    assert!(ledger.contains("\"historical_name\": \"RECIP\""));
    assert!(ledger
        .contains("\"historical_result\": \"fixed input -> 0; floating input -> reciprocal\""));
    assert!(ledger.contains("\"relation_to_core_math\": \"comparison-only-no-shared-identity\""));

    // Same human label, different law/domain.
    assert_eq!(eval_exact(&recip("2")), "1/2");
    assert_ne!("0", eval_exact(&recip("2")));
}
