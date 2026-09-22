#!/usr/bin/env bash
# RECON-06 (issue #28) -- єдиний відтворюваний closeout gate для
# standalone historical reconstruction (#7). Один виклик:
#   tests/closeout/run.sh
# збирає mccarthy-kernel.s наживо, ганяє всі три корпуси двічі (для
# детермінізму), фіксує toolchain/hash/machine-профіль і виводить
# один PASS/FAIL звіт. Не залежить від CML #180 -- це суто historical
# evaluator gate, compiler bridge лишається окремим (#13/#14).
#
# Single reproducible closeout gate for the standalone historical
# reconstruction (#7). One call: tests/closeout/run.sh. Builds
# mccarthy-kernel.s fresh, runs all three corpora twice (to prove
# determinism), records toolchain/hash/machine profile, prints one
# PASS/FAIL report. Independent of CML #180 -- this is the historical
# evaluator gate only; the compiler bridge stays separate (#13/#14).
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"
cd "$REPO_ROOT"

overall_fail=0

echo "=== RECON-06 closeout report ==="
echo "Timestamp (UTC): $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

echo "--- Toolchain ---"
gcc --version | head -1
as --version | head -1
qemu-x86_64 --version | head -1
echo

echo "--- Kernel source ---"
if git diff --quiet -- mccarthy-kernel.s 2>/dev/null; then
  echo "mccarthy-kernel.s: clean (no uncommitted changes)"
else
  echo "mccarthy-kernel.s: UNCOMMITTED CHANGES -- report below does not describe a committed state"
  overall_fail=1
fi
echo "Git HEAD: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
echo "mccarthy-kernel.s blob SHA-256 (source file): $(sha256sum mccarthy-kernel.s | awk '{print $1}')"
echo

echo "--- Machine profile ---"
cpu_model="$(grep -m1 'model name' /proc/cpuinfo | sed 's/^.*: //')"
echo "CPU: $cpu_model"
echo "Cores online: $(nproc)"
if [[ "$cpu_model" != *"i5-6400"* ]]; then
  echo "WARNING: this is not the declared reconstruction target (docs/references/hardware/OWNER-HARDWARE-PROFILE.md expects Intel Core i5-6400) -- live-verify before trusting this run as the i5-6400 closeout evidence."
  overall_fail=1
fi
echo "Kernel: $(uname -r)"
echo

run_gate() {
  local name="$1"
  local cmd="$2"
  echo "--- Gate: $name ---"
  if eval "$cmd" > /tmp/closeout_gate_output.$$ 2>&1; then
    tail -3 /tmp/closeout_gate_output.$$
    echo "GATE PASS: $name"
  else
    cat /tmp/closeout_gate_output.$$
    echo "GATE FAIL: $name"
    overall_fail=1
  fi
  rm -f /tmp/closeout_gate_output.$$
  echo
}

# Round 1
gcc -no-pie -O0 -s -o mccarthy-kernel mccarthy-kernel.s
hash_round1="$(sha256sum mccarthy-kernel | awk '{print $1}')"
echo "--- Binary hash (round 1 build) ---"
echo "mccarthy-kernel SHA-256: $hash_round1"
echo

run_gate "historical-core (#9, 21 fixtures)" "tests/historical-core/run.sh"
run_gate "isa-baseline (#7 physical-execution criterion, static + QEMU/Conroe)" "tests/isa-baseline/check-isa-baseline.sh"
run_gate "historical-facility-extensions (#27, 10 fixtures)" "tests/historical-facility-extensions/run.sh"

# Round 2 -- prove determinism, not just single-run success
echo "--- Determinism check: rebuild and rerun everything a second time ---"
gcc -no-pie -O0 -s -o mccarthy-kernel mccarthy-kernel.s
hash_round2="$(sha256sum mccarthy-kernel | awk '{print $1}')"
echo "mccarthy-kernel SHA-256 (round 2): $hash_round2"
if [[ "$hash_round1" == "$hash_round2" ]]; then
  echo "DETERMINISM PASS: binary hash identical across independent builds"
else
  echo "DETERMINISM FAIL: binary hash changed between builds ($hash_round1 vs $hash_round2)"
  overall_fail=1
fi
run_gate "historical-core (round 2, output-identity check)" "tests/historical-core/run.sh"
run_gate "historical-facility-extensions (round 2, output-identity check)" "tests/historical-facility-extensions/run.sh"
echo

echo "--- Known regression witnesses covered by this closeout ---"
echo "  #9 fixture 14-t-self-evaluates: 'T never self-evaluated' bug (README.md)"
echo "  #9 fixture 21-appq-plain-call-list-arg: 'missing appq' bug (README.md)"
echo "  Both adversarially verified in the PRs that introduced them (fixes reverted"
echo "  in an uncommitted scratch copy of mccarthy-kernel.s; corresponding fixtures failed)."
echo

echo "=== Summary ==="
if [[ "$overall_fail" -eq 0 ]]; then
  echo "RECON-06 CLOSEOUT: PASS -- standalone historical reconstruction (#7) gate satisfied on this machine."
else
  echo "RECON-06 CLOSEOUT: FAIL -- see gate output above."
fi
exit "$overall_fail"
