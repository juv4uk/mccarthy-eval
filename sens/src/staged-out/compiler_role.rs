//! Compiler-facing execution roles derived from SENS-owned semantic laws.
//!
//! This module does not define domain meaning and does not name backend
//! mechanisms. It exposes only a verified abstract role that downstream
//! compilers may bind to their own private mechanism references.

use crate::CoreDomainIdentity;

/// Backend-neutral execution role admitted by the current compiler slice.
///
/// These variants describe what SENS has already established semantically.
/// They are not CML/SLOT-VM/CUDA/FPGA opcodes or mechanism identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompilerExecutionRole {
    SelectorHead,
    SelectorTail,
    PairConstruct,
}

/// Backend-neutral compiler role for the complete current compiler-nucleus
/// semantic closure.  This type is representation only: production meaning is
/// derived by executing the SENS-owned structural law through
/// `compiler_lowering_role_from_sens`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompilerLoweringRole {
    QuoteForm,
    AtomPredicate,
    SelectorTail,
    SelectorHead,
    AtomEquality,
    CondForm,
    PairConstruct,
    LambdaForm,
    DefineForm,
}

/// Compatibility-facing mechanism adapter for SENS-owned compiler roles.
///
/// No D3/D4 bit patterns or language laws are decoded here. Execute the
/// language-owned compiler role projection and fail closed if it is unavailable.
/// This API is kept for existing GPU/host packet callers during migration.
pub fn compiler_execution_role(
    identity: CoreDomainIdentity,
) -> Option<CompilerExecutionRole> {
    crate::compiler_language::compiler_execution_role_from_sens(identity)
        .ok()
        .flatten()
}
