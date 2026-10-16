//! F3M-5: власна 5-байтова програма, закодована й виконана реальним кодеком.
//! Ім'я: "six-class-pick" — if atom(NIL)==atom(1) then CAR(CONS(NIL,NIL)) else CDR(CONS(NIL,NIL)).
//! Несе всі 6 capability-класів: COND, EQ, ATOM, CAR, CDR, CONS.
use sens::{
    eval_parsed_expressions, frame3_micro_enumerator::{decode_payload, encode_expr, Frame3MicroExpr as E},
    parse_canonical_binary, Session,
};

fn six_class_pick() -> E {
    let atom_nil = E::Atom(Box::new(E::Nil));
    let atom_one = E::Atom(Box::new(E::One));
    let eq = E::EqAtom(Box::new(atom_nil), Box::new(atom_one));
    let then_v = E::CarCons(Box::new(E::Nil), Box::new(E::Nil));
    let else_v = E::CdrCons(Box::new(E::Nil), Box::new(E::Nil));
    E::IfAtom(Box::new(eq), Box::new(then_v), Box::new(else_v))
}

#[test]
fn my_f3m5_program_encodes_to_five_bytes_and_round_trips() {
    let expr = six_class_pick();
    assert_eq!(expr.node_count(), 12, "must fit the 13-node F3M-5 window");
    let bytes = encode_expr(&expr).expect("encode");
    println!("six-class-pick bytes = {} ({})", bytes.len(), bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "));
    assert_eq!(bytes.len(), 5, "must be an F3M-5 payload (5 bytes)");
    let back = decode_payload(&bytes).expect("decode");
    assert_eq!(back, expr, "decode(encode(x)) == x");
}

#[test]
fn my_f3m5_program_executes_on_the_real_evaluator() {
    let expr = six_class_pick();
    let source = expr.exact_words().join(" ");
    println!("exact words = {source}");
    let parsed = parse_canonical_binary(&source).expect("canonical parse");
    let value = eval_parsed_expressions(&parsed, &mut Session::default()).expect("evaluate");
    println!("result = {}", value.value);
    // atom(NIL)=D1:1, atom(1)=D1:1 => EQ true => then-branch CAR(CONS(NIL,NIL)) = NIL
    assert_eq!(value.value.to_string(), "()", "expected structural NIL");
}
