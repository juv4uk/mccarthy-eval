#!/usr/bin/env bash
# Регресійний корпус для GET/DEFLIST/REMPROP (issue #7, property
# lists) -- Appendix B, LISP 1.5 Manual 1962 (ст.58-59), перевірено
# напряму по зображенню сторінки.
#
# Kernel вантажить startup.lisp за ВІДНОСНИМ шляхом, resolved проти
# CWD процесу -- запускається з $REPO_ROOT як CWD, як і всі інші
# tests/*/run.sh у цьому репо.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"

gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

pass=0
fail=0
for fixture in *.lisp; do
  base="${fixture%.lisp}"
  expected_file="${base}.expected"
  actual="$(cd "$REPO_ROOT" && timeout 5 "$REPO_ROOT/mccarthy-kernel" "$OLDPWD/$fixture" 2>/dev/null)"
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
