#!/usr/bin/env bash
# Regression corpus для adopted later historical facilities (issue #27,
# HISTORICAL-FACILITY-FIDELITY-1) -- PLUS/TIMES (n-ary, LISP 1.5 SS4.2
# p.31-32) і DIFFERENCE/ZEROP (уже source-faithful, регресія проти
# випадкового зламу). Окремий від tests/historical-core/ (#9) --
# historical-core corpus цей файл не чіпає й не залежить від нього.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"

gcc -no-pie -O0 -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

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
