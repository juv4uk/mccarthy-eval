// GENERATED — DO NOT EDIT BY HAND.
// Authority: lib/surface/function-signatures.lisp
// Generator: scripts/generate-rust-evaluator-dispatch.lisp

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NecessaryFormMechanism {
    Define,
    Lambda,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NecessaryFormDispatchRow {
    pub(super) semantic_id: u8,
    pub(super) mechanism: NecessaryFormMechanism,
}

pub(super) const NECESSARY_FORM_DISPATCH: &[NecessaryFormDispatchRow] = &[
    NecessaryFormDispatchRow { semantic_id: 0b00001000, mechanism: NecessaryFormMechanism::Lambda },
    NecessaryFormDispatchRow { semantic_id: 0b00001001, mechanism: NecessaryFormMechanism::Define },
    NecessaryFormDispatchRow { semantic_id: 0b00001011, mechanism: NecessaryFormMechanism::Define },
];
