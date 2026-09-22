# CML x86_64 freestanding backend — local snapshot

Source repository: juv4uk/cml
Source file: docs/x86-freestanding-backend.md
Source SHA: c7f61275c9e85f9c9299c9f11993cb9389e0fc0a
Additional current CML state checked: 2026-09-22

## What CML already does

CML is an AOT middle-end for my-lisp. Its freestanding backend is src/x86_freestanding.rs.

It consumes admitted shared Ir and emits deterministic GNU x86_64 assembly for the wsm-os target boundary.

The numeric value representation is imported from the pinned wsm-os-target contract, not copied from Rust my-lisp::Value.

## Current supported x86 slice

The documented backend supports:
- integer, () and t immediates;
- quoted integers and symbols;
- proper and dotted lists;
- cons, car, cdr, eq, atom through versioned wsm_* runtime ABI;
- cond with strict () identity;
- checked + and - fixnum arithmetic;
- loop-optimized self-tail-calls.

Unsupported IR fails during complete preflight before emission.

There is no libc fallback, syscall fallback, filesystem fallback, C backend fallback, or full my-lisp 3.0 claim.

## Entry / ABI

Documented conceptual entry:

Value wsm_entry(RuntimeContext *context)

The target uses SysV AMD64 conventions, returns the final value in rax, preserves runtime context in a callee-saved register, and maintains stack alignment across runtime calls.

Generated assembly is checked with a real GNU assembler. Undefined symbols are constrained to the wsm_* runtime imports admitted by the target contract.

## Meaning for our reconstruction

CML is already a viable x86-64 compiler substrate for a substantial Lisp subset.

It is not yet a complete compiler path for historical McCarthy apply/eval. The missing capability is documented separately in CML-HISTORICAL-EVAL-APPLY-GAP.md.

Authority:
- my-lisp owns language meaning and semantic identities;
- CML owns lowering, IR and emission;
- wsm-target-contract owns the shared machine ABI;
- mccarthy-eval owns the historical reconstruction witness.
