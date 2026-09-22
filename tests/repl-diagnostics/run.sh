#!/usr/bin/env bash
# Регресійний корпус для CONDITION-діагностики (2026-09-22) --
# перевіряє stdout і stderr ОКРЕМО, бо суть фічі саме в тому, що
# значення на stdout не змінюється (стара семантика Lisp: незв'язаний
# виклик -> NIL), а новий рядок з'являється лише на stderr. Формат
# натхненний реальним REPL-транскриптом wsm-os-lisp
# (`CONDITION kind=...`), не скопійований -- тут усе чистий асемблер.
set -euo pipefail
cd "$(dirname "$0")"
REPO_ROOT="$(cd ../.. && pwd)"

gcc -no-pie -O0 -s -o "$REPO_ROOT/mccarthy-kernel" "$REPO_ROOT/mccarthy-kernel.s"

pass=0
fail=0
for fixture in *.lisp; do
  base="${fixture%.lisp}"
  fixture_abs="$(pwd)/$fixture"
  actual_stdout="$(cd "$REPO_ROOT" && "$REPO_ROOT/mccarthy-kernel" "$fixture_abs" 2>/dev/null)"
  actual_stderr="$(cd "$REPO_ROOT" && "$REPO_ROOT/mccarthy-kernel" "$fixture_abs" 2>&1 1>/dev/null)"
  expected_stdout="$(cat "${base}.expected")"
  expected_stderr="$(cat "${base}.expected-stderr")"
  if [[ "$actual_stdout" == "$expected_stdout" && "$actual_stderr" == "$expected_stderr" ]]; then
    echo "PASS  $fixture"
    pass=$((pass + 1))
  else
    echo "FAIL  $fixture"
    echo "  stdout expected=[$expected_stdout] actual=[$actual_stdout]"
    echo "  stderr expected=[$expected_stderr] actual=[$actual_stderr]"
    fail=$((fail + 1))
  fi
done

echo "---"
echo "$pass passed, $fail failed"
[[ "$fail" -eq 0 ]]
