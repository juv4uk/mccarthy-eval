// GENERATED — DO NOT EDIT BY HAND.
// Authority: lib/surface/d5-definition-bindings.lisp
// Checked by: scripts/check-d5-definition-bindings.py

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct D5DefinitionBinding {
    pub(super) name: &'static str,
    pub(super) bits: u8,
}

pub(super) const D5_DEFINITION_BINDINGS: &[D5DefinitionBinding] = &[
    D5DefinitionBinding { name: "reverse", bits: 0b10100 },
    D5DefinitionBinding { name: "reverse-onto", bits: 0b10101 },
    D5DefinitionBinding { name: "quotient", bits: 0b10111 },
    D5DefinitionBinding { name: "assoc", bits: 0b11100 },
    D5DefinitionBinding { name: "member?", bits: 0b11101 },
    D5DefinitionBinding { name: "subst", bits: 0b11111 },
];
