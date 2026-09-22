# WSM x86-64 target contract — local reference

Source repository: juv4uk/wsm-target-contract
Source file: README.md
Source SHA: 184e54caf18a9761f6ad6d23cecab742eac8937f
Snapshot: 2026-09-22

## Role

wsm-target-contract is the neutral shared machine boundary for the first WSM x86_64 target.

Two equal consumers are documented:
- CML — emits code that must obey the contract.
- wsm-os-lisp — supplies runtime/platform execution for that contract.

The repository is intentionally neither compiler nor runtime.

## Contract scope

The shared boundary covers:
- value words and tags;
- cons alignment;
- closure descriptor alignment;
- x86_64 calling ABI;
- allowed runtime import names.

## Authority boundaries

my-lisp defines language semantics.
CML defines parsing, admission, IR and emission.
wsm-os-lisp defines heap, boot and platform behavior.
wsm-target-contract only fixes the shared machine boundary.

For mccarthy-eval this contract says how generated code reaches the machine. It does not say what McCarthy historical semantics should mean.

Upstream verification:
cargo test -p wsm-os-target

The local reconstruction should record the exact target-contract revision used by every future compiler witness.
