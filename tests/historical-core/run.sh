#!/usr/bin/env bash
# Regression corpus для historical-core: apply/appq/eval/evcon/evlis,
# семи диспетчерів (QUOTE/ATOM/EQ/COND/CAR/CDR/CONS), LABEL/LAMBDA та
# environment/assoc-поведінки. Кожен .lisp fixture тут навмисно НЕ
# використовує DEFINE, арифметику чи REPL -- лише сім примітивів
# Маккарті 1960 р. плюс LABEL/LAMBDA, як у самій формулі eval.
#
# Regression corpus for the historical core: apply/appq/eval/evcon/
# evlis, the seven dispatchers (QUOTE/ATOM/EQ/COND/CAR/CDR/CONS), and
# LABEL/LAMBDA/environment-assoc behavior. Every .lisp fixture here
# deliberately avoids DEFINE, arithmetic, and the REPL -- only
# McCarthy 1960's seven primitives plus LABEL/LAMBDA, exactly as in
# the eval formula itself.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"

gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

pass=0
fail=0
for fixture in *.lisp; do
  base="${fixture%.lisp}"
  expected_file="${base}.expected"
  if [[ ! -f "$expected_file" ]]; then
    echo "MISSING EXPECTED: $fixture"
    fail=$((fail + 1))
    continue
  fi
  actual="$(cd "$REPO_ROOT" && "$REPO_ROOT/mccarthy-kernel" "$OLDPWD/$fixture")"
  expected="$(cat "$expected_file")"
  if [[ "$actual" == "$expected" ]]; then
    echo "PASS  $fixture"
    pass=$((pass + 1))
  else
    echo "FAIL  $fixture"
    echo "  expected: $expected"
    echo "  actual:   $actual"
    fail=$((fail + 1))
  fi
done

echo "---"
echo "$pass passed, $fail failed"
[[ "$fail" -eq 0 ]]
