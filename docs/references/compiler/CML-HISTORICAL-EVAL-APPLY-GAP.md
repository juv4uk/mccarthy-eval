# CML gap for full historical eval/apply

Snapshot: 2026-09-22

External repository: juv4uk/cml
Tracked task: CML #180 — X86-HISTORICAL-LISP: довести general first-class application для McCarthy eval/apply

## Source-confirmed capability boundary

Current CML x86 tests/documentation show support for:
- named fixed-arity definitions;
- ordinary recursion in supported bounded forms;
- self-tail recursion;
- bounded direct lambda application;
- bounded escaping closure forms.

But a real call graph through my-apply / my-eval-application-result can fail x86 freestanding preflight because it performs application through a first-class function value selected at runtime.

The current backend is fail-closed. General dynamic application is not yet an unrestricted capability.

## Why historical reconstruction needs it

The 1960 published equations contain:

apply[f;args] = eval[cons[f; appq[args]]; NIL]

Therefore the complete path must ultimately support:

function value
→ apply
→ eval

rather than only statically known function names.

## Required compiler work

CML #180 is scoped to:
1. explicit runtime callable representation;
2. bounded fail-closed first-class application;
3. preserved current closure and recursion guarantees;
4. differential oracle/host/QEMU evidence;
5. no hidden compiler-specific semantic fallback.

## What mccarthy-eval must not do

Do not solve the CML gap by copying an evaluator into Rust or by embedding a special hand-written evaluator inside CML.

Before #180 is closed, this repository can:
- finish the standalone historical reconstruction;
- build the historical regression corpus;
- document the exact unsupported compiler shape;
- prepare differential fixtures.

It cannot honestly claim that the CML route is already a complete compilation path for historical apply/eval.

This file is a compiler capability snapshot, not historical evidence and not language authority.
