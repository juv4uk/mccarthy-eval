# my-lisp → Intel Core i5-6400 / x86-64

Sources:
- juv4uk/my-lisp/lib/machine/cpu/intel-core-i5-6400.lisp
  SHA 1b4f98e86f8e9fac14dcc74862bf82afd536c8a3
- juv4uk/my-lisp/lib/machine/cpu/intel-core-i5-6400-inventory.lisp
  SHA 0565835ee06eacd0574df3bc35aab52b5c9b2573
- juv4uk/my-lisp/docs/superpowers/specs/2026-09-14-vertical-lisp-isa-design.md
  SHA 47992efb7a9e7bd3ca0d7fe448c26602e1c79572

Snapshot: 2026-09-22

## CPU profile

my-lisp has an explicit target profile:

cpu = intel-core-i5-6400
microarchitecture = skylake
isa = x86-64
mode = 64-bit

Baseline extensions:
X86-BASE, X86-64, X87, MMX, SSE, SSE2, SSE3, SSSE3, SSE4.1, SSE4.2

Gated extensions:
AES-NI, PCLMULQDQ, AVX, F16C, FMA3, BMI1, BMI2, AVX2, RDRAND, RDSEED, ADX, XSAVE, CLFLUSHOPT

Platform-gated:
MPX, SGX

Unavailable:
TSX, AVX-512, AMX

The profile requires user-mode legality and runtime feature checks. AVX state additionally requires XGETBV.

## Skylake inventory

The ratified inventory identifies:

cpu = intel-core-i5-6400
microarchitecture = skylake
profile-class = client
admission-policy = fail-closed

It explicitly avoids server-SKU AVX-512 assumptions.

## Modern Lisp machine stack

The vertical my-lisp design is:

semantic registry
→ semantic lowering
→ structured machine forms
→ closed admission
→ Lisp-owned encoder
→ machine bytes
→ host execution mechanism
→ physical CPU

Relevant machine layers:
- lib/machine/isa/
- lib/machine/cpu/
- lib/machine/operands/
- lib/machine/admission/
- lib/machine/encoding/
- lib/machine/lowering/

The machine layer is not a second semantic authority.

## Why this matters to mccarthy-eval

The modern ecosystem already describes and witnesses the exact physical CPU used by this historical reconstruction.

That is useful as a machine/target fact, but it must not rewrite historical McCarthy semantics.

Correct direction:

historical Lisp
→ historical witness
→ later my-lisp/CML projection
→ same physical x86-64 CPU

Not:

my-lisp semantics
→ rewrite history.
