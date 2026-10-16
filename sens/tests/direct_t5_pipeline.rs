//! Physical T5 and the human bit-view must use the SAME D2 and evaluator
//! laws, but the production path must not construct the bit-view at all.
use sens::{
    decode_ternary_words, encode_ternary_words, eval_parsed_expressions,
    eval_t5_program, open_ternary_program, parse_canonical_binary,
    PhysicalT5Program, Session, T5ExecutionError, TernaryTransportError,
};

const QUOTE: &[u8] = include_bytes!("../../../tests/fixtures/migration-quote-cohort-main/quote-legacy.sens");
const ATOM: &[u8] = include_bytes!("../../../tests/fixtures/migration-atom-cohort/atom-empty.sens");
const COND: &[u8] = include_bytes!("../../../tests/fixtures/migration-d1-cond-cohort/branch.sens");
const MULTIFORM: &[u8] = include_bytes!("../../../tests/fixtures/migration-multiform-cohort-main/two-forms.sens");

#[test]
fn physical_execute_matches_reference_visible_path_for_admitted_fixtures() {
    for (label, physical) in [("quote", QUOTE), ("atom", ATOM), ("cond", COND), ("multi", MULTIFORM)] {
        let program = PhysicalT5Program::decode(physical).expect(label);
        assert!(program.form_count() > 0, "{label}");
        assert_eq!(program.form_count(), if label == "multi" { 2 } else { 1 });

        // Comparison/reference route only. Production never renders this.
        let visible = open_ternary_program(physical).expect(label);
        let parsed = parse_canonical_binary(&visible).expect(label);
        let reference = eval_parsed_expressions(&parsed, &mut Session::default())
            .expect(label);
        let direct = program.execute(&mut Session::default()).expect(label);
        let one_shot = eval_t5_program(physical, &mut Session::default()).expect(label);
        assert_eq!(direct.value, reference.value, "{label} result law");
        assert_eq!(direct.output, reference.output, "{label} observable output");
        assert_eq!(one_shot.value, direct.value, "{label} one-shot parity");
        assert_eq!(one_shot.output, direct.output, "{label} one-shot output");
        let again = program.execute(&mut Session::default()).expect(label);
        assert_eq!(again.value, direct.value, "{label} repeat decoded program");
    }
}

#[test]
fn all_invalid_physical_or_d2_inputs_are_rejected_before_interpreter() {
    for invalid in [&[][..], &[243][..], &[242][..], &[100, 242][..]] {
        assert!(matches!(
            PhysicalT5Program::decode(invalid),
            Err(T5ExecutionError::Transport(_)),
        ));
    }
    // The physical word 01 is a D2 CLOSE, not an executable value.
    let close = decode_ternary_words(&encode_ternary_words(&[
        sens::parse_binary_source_words("01").unwrap()[0].word
    ]).unwrap()).unwrap();
    let bad_structure = encode_ternary_words(&close).unwrap();
    assert!(matches!(
        PhysicalT5Program::decode(&bad_structure),
        Err(T5ExecutionError::Language(_))
    ));
    assert!(matches!(
        eval_t5_program(&bad_structure, &mut Session::default()),
        Err(T5ExecutionError::Language(_))
    ));
    assert_eq!(decode_ternary_words(&[243]), Err(TernaryTransportError::InvalidPhysicalByte));
}

#[test]
fn exact_widths_remain_distinct_before_any_surface_lookup() {
    let a = sens::encode_binary_projection_ternary("1").unwrap();
    let b = sens::encode_binary_projection_ternary("001").unwrap();
    let c = sens::encode_binary_projection_ternary("000000001").unwrap();
    assert_ne!(a, b);
    assert_ne!(b, c);
    for physical in [&a, &b, &c] {
        assert_eq!(PhysicalT5Program::decode(physical).unwrap().form_count(), 1);
    }
}
