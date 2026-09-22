#!/usr/bin/env bash
# Регресійний корпус для LISP 1.5 Appendix A primitives, що
# "стосуються заліза" (issue #7, Фаза 2) -- реальні x86-64
# інструкції (idiv/imul/and/or/xor/shl/sar/cmov), не LABEL/LAMBDA
# library-функції (ті окремо, поки не реалізовані). Окремий від
# tests/historical-core/ (#9) і tests/historical-facility-extensions/
# (#27) -- жоден з них не чіпається.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"

gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

pass=0
fail=0
for fixture in *.lisp; do
  base="${fixture%.lisp}"
  expected_file="${base}.expected"
  actual="$("$REPO_ROOT/mccarthy-kernel" "$fixture")"
  expected="$(cat "$expected_file")"
  if [[ "$actual" == "$expected" ]]; then
    echo "PASS  $fixture"
    pass=$((pass + 1))
  else
    echo "FAIL  $fixture  expected=[$expected] actual=[$actual]"
    fail=$((fail + 1))
  fi
done

echo "---"
echo "$pass passed, $fail failed"
[[ "$fail" -eq 0 ]]
