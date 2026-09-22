#!/usr/bin/env bash
# Доказ (не декларація) того, що mccarthy-kernel.s -- чистий скалярний
# x86-64 і не вимагає жодного з gated-розширень цього CPU (issue #7,
# «Correctness path should prefer scalar x86-64 and must not require
# AVX2/BMI2 or other optional extensions»).
#
# Два незалежні докази:
# 1. Статичний: дизасемблюємо реальний бінарник і шукаємо мнемоніки,
#    заявлені як належні gated-розширенням у my-lisp/lib/machine/isa/*
#    (авторитетне джерело ISA-каталогу -- не власний здогад), плюс
#    структурний пошук будь-якої VEX-кодованої інструкції (усі форми
#    AVX/AVX2/FMA3/F16C завжди мають мнемоніку з префіксом V).
# 2. Динамічний, найсильніший: увесь historical-core корпус (tests/
#    historical-core/) запускається під QEMU з емуляцією Intel Core 2
#    "Conroe" (2006 р.) -- CPU, що ФІЗИЧНО не має SSE4.x/AVX/BMI/AES-NI
#    взагалі. Якщо кожен fixture дає той самий результат, що на
#    реальному i5-6400 -- це не "в бінарнику не видно інструкції", а
#    "бінарник справді виконується без цієї апаратної спроможності".
#
# Proof (not a claim) that mccarthy-kernel.s is plain scalar x86-64 and
# requires none of this CPU's gated extensions (issue #7). Two
# independent proofs: (1) static -- disassemble the real binary and
# search for mnemonics that my-lisp/lib/machine/isa/*.lisp (the
# authoritative ISA catalogue, not a guess) assigns to each gated
# extension, plus a structural scan for any VEX-encoded instruction;
# (2) dynamic, strongest -- run the whole historical-core corpus under
# QEMU emulating an Intel Core 2 "Conroe" (2006), a CPU with no
# SSE4.x/AVX/BMI/AES-NI at all. Matching output there is proof of
# actual execution without the capability, not just an absent mnemonic.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"
CORPUS_DIR="$REPO_ROOT/tests/historical-core"

gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

DISASM="$(mktemp)"
trap 'rm -f "$DISASM"' EXIT
objdump -d -M intel "$REPO_ROOT/mccarthy-kernel" > "$DISASM"

# Мнемоніки -- дослівно з my-lisp/lib/machine/isa/*.lisp catalogue
# (blob SHA кожного файлу зафіксовано в PROVENANCE.md цієї директорії).
declare -A CATALOGUE
CATALOGUE[AES-NI]="aesenc aesdec aeskeygenassist"
CATALOGUE[PCLMULQDQ]="pclmulqdq"
CATALOGUE[AVX]="vaddps vxorps"
CATALOGUE[F16C]="vcvtph2ps vcvtps2ph"
CATALOGUE[FMA3]="vfmadd132ps vfmadd213ps vfmadd231ps"
CATALOGUE[BMI1]="andn bextr blsi blsmsk blsr tzcnt"
CATALOGUE[BMI2]="pdep pext mulx shlx shrx sarx"
CATALOGUE[AVX2]="vpaddd vpmulld"
CATALOGUE[RDRAND]="rdrand"
CATALOGUE[RDSEED]="rdseed"
CATALOGUE[ADX]="adcx adox"
CATALOGUE[XSAVE]="xsave xrstor"
CATALOGUE[CLFLUSHOPT]="clflushopt"

static_fail=0
for ext in "${!CATALOGUE[@]}"; do
  for mnem in ${CATALOGUE[$ext]}; do
    if grep -qwi "$mnem" "$DISASM"; then
      echo "STATIC FAIL: found $mnem ($ext) in mccarthy-kernel"
      static_fail=1
    fi
  done
done
if grep -qiE '\bv[a-z0-9]+\s' "$DISASM"; then
  echo "STATIC FAIL: found a VEX-encoded (V-prefixed) instruction -- AVX/AVX2/FMA3/F16C family"
  static_fail=1
fi
if grep -qiE '%?[xyz]mm[0-9]+' "$DISASM"; then
  echo "STATIC FAIL: found xmm/ymm/zmm register usage"
  static_fail=1
fi
if [[ "$static_fail" -eq 0 ]]; then
  echo "STATIC PASS: no gated-extension mnemonic, no VEX prefix, no xmm/ymm/zmm register in mccarthy-kernel"
fi

echo "---"

qemu_pass=0
qemu_fail=0
for fixture in "$CORPUS_DIR"/*.lisp; do
  base="${fixture%.lisp}"
  expected_file="${base}.expected"
  actual="$(cd "$REPO_ROOT" && qemu-x86_64 -cpu Conroe "$REPO_ROOT/mccarthy-kernel" "$fixture" 2>/dev/null)"
  expected="$(cat "$expected_file")"
  if [[ "$actual" == "$expected" ]]; then
    qemu_pass=$((qemu_pass + 1))
  else
    qemu_fail=$((qemu_fail + 1))
    echo "QEMU/Conroe FAIL: $(basename "$fixture")  expected=[$expected] actual=[$actual]"
  fi
done
echo "QEMU/Conroe (2006 Core 2, no SSE4.x/AVX/BMI/AES-NI): $qemu_pass passed, $qemu_fail failed"

[[ "$static_fail" -eq 0 && "$qemu_fail" -eq 0 ]]
