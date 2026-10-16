//! #3484 prerequisite probe: exact D4 APPLY/EVAL before D5 EVALQUOTE/FUNCTION.
//! Research-only; no semantic authority change.

use sens::{eval_parsed_expressions, eval_program, load_core_library, Bit4, CoreD4, CoreDomainIdentity, Expr, ExprKind, Session, Span};

fn run_exact(bits: u8, args_source: &[&str]) -> Result<String,String> {
    let mut session=Session::default();
    load_core_library(&mut session).map_err(|e| format!("load-core: {e:?}"))?;
    let id=CoreDomainIdentity::D4(CoreD4::from_word(Bit4::new(bits).unwrap()));
    let mut args=Vec::new();
    for src in args_source {
        let mut parsed=sens::parse(src).map_err(|e| format!("parse {src}: {e:?}"))?;
        if parsed.len()!=1 { return Err(format!("expected one expr for {src}")); }
        args.push(parsed.remove(0));
    }
    let form=Expr{
        kind:ExprKind::DomainCall(id, args.into()),
        span:Span{start:0,end:0},
    };
    eval_parsed_expressions(&[form], &mut session)
        .map(|r| r.value.to_string())
        .map_err(|e| format!("{:?}: {}", e.kind, e.message))
}

fn run_surface(source:&str)->Result<String,String>{
    let mut session=Session::default();
    load_core_library(&mut session).map_err(|e| format!("load-core: {e:?}"))?;
    eval_program(source,&mut session)
        .map(|r| r.value.to_string())
        .map_err(|e| format!("{:?}: {}",e.kind,e.message))
}

#[test]
fn print_exact_d4_apply_eval_matrix(){
    let cases=[
        ("D4-APPLY-symbol", run_exact(0b0000, &["(quote +)","(quote (2 3))"])),
        ("D4-APPLY-lambda-form", run_exact(0b0000, &["(quote (lambda (x) (+ x 1)))","(quote (4))"])),
        ("D4-EVAL-number", run_exact(0b0001, &["(quote (+ 2 3))"])),
        ("D4-EVAL-lambda", run_exact(0b0001, &["(quote (lambda (x) (+ x 1)))"])),
        ("surface-apply", run_surface("(apply (quote +) (quote (2 3)))")),
        ("surface-eval", run_surface("(eval (quote (+ 2 3)))")),
    ];
    for (name,result) in cases { println!("D4-EVAL-APPLY-PROBE {name}={result:?}"); }
}

#[test]
fn exact_d4_lambda_capture_control(){
    let result=run_surface("((lambda (x) ((lambda (y) (+ x y)) 3)) 4)")
        .expect("current closure capture control");
    assert_eq!(result,"7");
}
