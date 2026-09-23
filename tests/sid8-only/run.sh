#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"
gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"
pass=0
fail=0
for fixture in *.lisp; do
  base="${fixture%.lisp}"
  actual="$(cd "$REPO_ROOT" && "$REPO_ROOT/mccarthy-kernel" "$OLDPWD/$fixture")"
  expected="$(cat "${base}.expected")"
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
