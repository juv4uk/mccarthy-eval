#!/usr/bin/env python3
"""Run the reproducible mccarthy-eval ↔ CML differential witness.

This runner compares actual outputs only. It never embeds expected Lisp answers
or evaluator semantics. CML is consumed from a clean, exact pinned checkout;
the temporary Rust driver calls CML's own parser/lower/backend/witness APIs.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


DEFAULT_CML_SHA = "2a3e59fe04cc2d8a8d73c0bfbb5860bdb1f39854"
TRANSLATED_FIXTURES = {
    "19-label-recursion-mylen": Path("tests/cml-bridge/translated/19-label-recursion-mylen.lisp"),
}


class RunnerError(RuntimeError):
    pass


def run(cmd: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def checked(cmd: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> str:
    result = run(cmd, cwd=cwd, env=env)
    if result.returncode != 0:
        raise RunnerError(
            f"command failed ({result.returncode}): {' '.join(cmd)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result.stdout.strip()


def git(repo: Path, *args: str) -> str:
    return checked(["git", "-C", str(repo), *args])


def git_blob_sha(repo: Path, path: Path) -> str:
    return git(repo, "hash-object", str(path))


def fixture_metadata(matrix_path: Path, provenance_path: Path, fixture_base: str) -> tuple[str, str]:
    text = matrix_path.read_text(encoding="utf-8")
    provenance_text = provenance_path.read_text(encoding="utf-8")
    matches = []
    for line in text.splitlines():
        if not line.startswith("|") or line.startswith("|---"):
            continue
        columns = [column.strip() for column in line.split("|")[1:-1]]
        if len(columns) != 4:
            continue
        matrix_fixtures = {
            token.strip()
            for token in re.split(r"[/,]", columns[0])
            if token.strip()
        }
        if fixture_base in matrix_fixtures:
            matches.append((columns[2], columns[3]))

    if len(matches) != 1:
        raise RunnerError(
            f"admission matrix must contain exactly one row for {fixture_base}, got {len(matches)}"
        )

    _, status = matches[0]

    provenance_matches = []
    for line in provenance_text.splitlines():
        if line.startswith("|") and fixture_base in line and "source-confirmed" in line or "reconstruction-derived witness" in line or "regression witness" in line:
            provenance_matches.append(line)

    if not provenance_matches:
        raise RunnerError(f"historical provenance is not recorded for {fixture_base}")

    provenance_labels = []
    for line in provenance_matches:
        if "source-confirmed" in line:
            provenance_labels.append("source-confirmed")
        if "reconstruction-derived witness" in line:
            provenance_labels.append("reconstruction-derived witness")
        if "regression witness" in line:
            provenance_labels.append("regression witness")

    provenance_labels = sorted(set(provenance_labels))
    if len(provenance_labels) != 1:
        raise RunnerError(
            f"historical provenance for {fixture_base} is ambiguous: {provenance_labels}"
        )

    return (
        provenance_labels[0],
        "blocked" if "**blocked" in status else "candidate",
    )

def translation_source_for(fixture_base: str, repo_root: Path, translations_path: Path) -> Path | None:
    translated = TRANSLATED_FIXTURES.get(fixture_base)
    if translated is None:
        return None
    metadata = translations_path.read_text(encoding="utf-8")
    if fixture_base not in metadata:
        raise RunnerError(f"translation metadata does not mention {fixture_base}")
    path = repo_root / translated
    if not path.is_file():
        raise RunnerError(f"translation source is missing: {path}")
    return path


def sanitize_output(value: str) -> str:
    return value.rstrip("\n")


def envelope_historical(actual: str) -> str:
    escaped = actual.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    return f'(value "{escaped}")'


def parse_driver_output(stdout: str) -> tuple[str, str]:
    line = stdout.strip().splitlines()[-1] if stdout.strip() else ""
    status, sep, value = line.partition("\t")
    if not sep:
        raise RunnerError(f"CML driver emitted malformed output: {stdout!r}")
    return status, value.replace("\\t", "\t").replace("\\n", "\n").replace("\\\\", "\\")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--cml-repo", type=Path, default=Path(os.environ.get("CML_REPO", "../cml")))
    parser.add_argument(
        "--wsm-my-lisp-repo",
        type=Path,
        default=Path(os.environ.get("WSM_MY_LISP_REPO", "../wsm-my-lisp")),
    )
    parser.add_argument("--cml-sha", default=os.environ.get("CML_SHA", DEFAULT_CML_SHA))
    parser.add_argument("--output", type=Path, default=None)
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    cml_repo = args.cml_repo.resolve()
    wsm_repo = args.wsm_my_lisp_repo.resolve()

    if not (repo_root / ".git").exists():
        raise RunnerError(f"not a git checkout: {repo_root}")
    if not (cml_repo / ".git").exists():
        raise RunnerError(f"not a git checkout: {cml_repo}")
    if not (wsm_repo / ".git").exists():
        raise RunnerError(f"not a git checkout: {wsm_repo}")

    cml_head = git(cml_repo, "rev-parse", "HEAD")
    if cml_head != args.cml_sha:
        raise RunnerError(
            f"CML checkout is not pinned: expected {args.cml_sha}, got {cml_head}"
        )
    if git(cml_repo, "status", "--porcelain"):
        raise RunnerError("CML checkout is dirty; exact-commit evidence requires a clean tree")

    my_lisp_repo = cml_repo / "external" / "my-lisp"
    if not (my_lisp_repo / ".git").exists():
        raise RunnerError(f"CML my-lisp checkout is not initialized: {my_lisp_repo}")
    my_lisp_sha = git(my_lisp_repo, "rev-parse", "HEAD")
    wsm_sha = git(wsm_repo, "rev-parse", "HEAD")
    nucleus = (wsm_repo / "asm" / "nucleus.s").resolve()
    if not nucleus.is_file():
        raise RunnerError(f"missing x86 nucleus: {nucleus}")

    matrix = repo_root / "docs" / "references" / "compiler" / "CML-HISTORICAL-ADMISSION-MATRIX.md"
    provenance = repo_root / "tests" / "historical-core" / "PROVENANCE.md"
    translations = repo_root / "tests" / "cml-bridge" / "TRANSLATIONS.md"
    driver_source = repo_root / "tests" / "cml-bridge" / "cml-witness-driver.rs"
    if not matrix.is_file() or not provenance.is_file() or not translations.is_file() or not driver_source.is_file():
        raise RunnerError("required CML bridge metadata/driver file is missing")

    with tempfile.TemporaryDirectory(prefix="mccarthy-cml-31-") as tmp:
        tmp_root = Path(tmp)
        kernel = tmp_root / "mccarthy-kernel"
        cargo_project = tmp_root / "cml-driver"
        cargo_project.mkdir()
        (cargo_project / "src").mkdir()

        compile_kernel = run(
            ["gcc", "-no-pie", "-O0", "-o", str(kernel), str(repo_root / "mccarthy-kernel.s")]
        )
        if compile_kernel.returncode != 0:
            raise RunnerError(
                f"historical kernel build failed:\n{compile_kernel.stdout}\n{compile_kernel.stderr}"
            )

        manifest = f"""[package]
name = "mccarthy_cml_witness_driver"
version = "0.0.0"
edition = "2024"

[dependencies]
cml = {{ path = {json.dumps(str(cml_repo))} }}
"""
        (cargo_project / "Cargo.toml").write_text(manifest, encoding="utf-8")
        shutil.copy2(driver_source, cargo_project / "src" / "main.rs")

        env = os.environ.copy()
        env["WSM_NUCLEUS_ASM"] = str(nucleus)
        env["CARGO_TARGET_DIR"] = str(tmp_root / "cargo-target")
        build = run(["cargo", "build", "--release"], cwd=cargo_project, env=env)
        if build.returncode != 0:
            raise RunnerError(
                f"CML driver build failed:\n{build.stdout}\n{build.stderr}"
            )

        driver_bin = Path(env["CARGO_TARGET_DIR"]) / "release" / "mccarthy_cml_witness_driver"
        if not driver_bin.is_file():
            raise RunnerError(f"CML driver binary missing: {driver_bin}")

        fixture_paths = sorted(repo_root.glob("tests/historical-core/*.lisp"))
        if len(fixture_paths) != 21:
            raise RunnerError(f"historical corpus must contain 21 fixtures, found {len(fixture_paths)}")

        records: list[dict[str, object]] = []
        for fixture in fixture_paths:
            fixture_base = fixture.stem
            historical_provenance, matrix_status = fixture_metadata(matrix, provenance, fixture_base)
            fixture_sha = git_blob_sha(repo_root, fixture)
            historical = run([str(kernel), str(fixture)], cwd=repo_root)
            if historical.returncode != 0:
                raise RunnerError(
                    f"Witness A failed for {fixture_base}: "
                    f"{historical.returncode}\n{historical.stderr}"
                )
            historical_actual = sanitize_output(historical.stdout)
            translation = translation_source_for(fixture_base, repo_root, translations)
            source_path = translation or fixture
            translation_sha = git_blob_sha(repo_root, translation) if translation else ""

            record: dict[str, object] = {
                "fixture": str(fixture.relative_to(repo_root)),
                "fixture_sha": fixture_sha,
                "historical_provenance": historical_provenance,
                "cml_source": (
                    str(source_path.relative_to(repo_root))
                    if matrix_status != "blocked"
                    else None
                ),
                "translation_sha": translation_sha or None,
                "my_lisp_sha": my_lisp_sha,
                "cml_sha": cml_head,
                "wsm_my_lisp_sha": wsm_sha,
                "target": "native-x86-64",
                "witness_a_output": historical_actual,
                "witness_b_output": None,
                "status": None,
                "notes": [],
                "qemu_status": "not-run",
                "qemu_reason": "CML witness bridge currently executes its native child process; no QEMU executor is exposed by the pinned CML API.",
            }

            if matrix_status == "blocked":
                record["status"] = "blocked"
                record["notes"] = ["Explicitly blocked by the current admission matrix; separate witness issue required."]
                records.append(record)
                continue

            cml_run = run([str(driver_bin), str(source_path)], cwd=repo_root, env=env)
            status, value = parse_driver_output(cml_run.stdout)
            if status == "ok":
                record["witness_b_output"] = value
                expected_envelope = envelope_historical(historical_actual)
                if value == expected_envelope:
                    record["status"] = "match"
                else:
                    record["status"] = "mismatch"
                    record["notes"] = [
                        "Raw actual outputs differ; no compatibility normalization was applied."
                    ]
            elif status == "unsupported":
                record["status"] = "unsupported"
                record["witness_b_output"] = value
            else:
                record["status"] = "error"
                record["witness_b_output"] = value
                record["notes"] = ["CML path failed before producing a canonical actual result."]
                if cml_run.returncode == 0:
                    raise RunnerError(
                        f"CML driver reported error with zero exit status for {fixture_base}: {value}"
                    )

            records.append(record)

        if args.output is None:
            for record in records:
                print(json.dumps(record, ensure_ascii=False, sort_keys=True))
        else:
            output = args.output.resolve()
            output.parent.mkdir(parents=True, exist_ok=True)
            with output.open("w", encoding="utf-8") as stream:
                for record in records:
                    stream.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RunnerError as error:
        print(f"cml differential runner: {error}", file=sys.stderr)
        raise SystemExit(2)
