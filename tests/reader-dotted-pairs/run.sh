#!/usr/bin/env bash
# Регресійний корпус для reader-fix issue #7 ("reader dot-notation")
# -- kernel's reader тепер парсить "(A . B)" на вході, симетрично до
# того, що printer уже давно виробляє на виході. Раніше (X . 1) на
# вході читалось як звичайний 3-елементний список (X, символ ".", 1),
# не як справжня cons-пара -- задокументовано в
# tests/lisp15-library-functions/PROVENANCE.md, тепер виправлено.
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
