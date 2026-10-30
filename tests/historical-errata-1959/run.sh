#!/usr/bin/env bash
# Виконуваний свідок саме механізму AIM-008 errata: SUBST заходить у
# QUOTE, тоді як SUBSQ зупиняє підстановку. Це не повний eval 1959 року.
set -euo pipefail
cd "$(dirname "$0")"
test_dir="$PWD"
repo_root="$(cd ../.. && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT

gcc -no-pie -O0 -s -o "$repo_root/mccarthy-kernel" "$repo_root/mccarthy-kernel.s"

run_and_check() {
  local fixture="$1" expected="$2" evaluated_expected="$3" label="$4"
  local result
  result="$(cd "$repo_root" && "$repo_root/mccarthy-kernel" "$test_dir/$fixture")"
  if [[ "$result" != "$(cat "$expected")" ]]; then
    printf 'FAIL %s substitution expected=[%s] actual=[%s]\n' \
      "$label" "$(cat "$expected")" "$result" >&2
    exit 1
  fi
  printf '%s\n' "$result" > "$tmp/$label-evaluation.lisp"
  local evaluated
  evaluated="$(cd "$repo_root" && "$repo_root/mccarthy-kernel" "$tmp/$label-evaluation.lisp")"
  if [[ "$evaluated" != "$(cat "$evaluated_expected")" ]]; then
    printf 'FAIL %s evaluation expected=[%s] actual=[%s]\n' \
      "$label" "$(cat "$evaluated_expected")" "$evaluated" >&2
    exit 1
  fi
  printf 'PASS %s substituted=%s evaluated=%s\n' "$label" "$result" "$evaluated"
}

run_and_check 01-naive-subst-enters-quote.lisp \
  01-naive-subst-enters-quote.expected 01-naive-subst-evaluated.expected naive-subst
run_and_check 02-subsq-stops-at-quote.lisp \
  02-subsq-stops-at-quote.expected 02-subsq-evaluated.expected corrected-subsq
