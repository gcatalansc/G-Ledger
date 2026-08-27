"""Opt-in, one-shot artifact adapter; the GSP compiler remains external and unchanged."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

MAX_INPUT_BYTES = 1_048_576
MAX_OUTPUT_BYTES = 8_388_608
GSP_ROOT = Path(__file__).resolve().parents[1]


class Rejected(Exception):
    """A data-free rejection code suitable for the process error channel."""


def git(source: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args],
        capture_output=True,
        check=False,
        timeout=10,
    )
    if result.returncode:
        raise Rejected("SOURCE_GIT_CHECK_FAILED")
    return result.stdout.decode("utf-8").strip()


def verify_source(source: Path, lock: dict) -> Path:
    if not source.is_absolute() or source.is_symlink() or not source.is_dir():
        raise Rejected("SOURCE_PATH_INVALID")
    source = source.resolve(strict=True)
    if Path(git(source, "rev-parse", "--show-toplevel")).resolve() != source:
        raise Rejected("SOURCE_ROOT_MISMATCH")
    if git(source, "rev-parse", "HEAD") != lock["implementation_commit"]:
        raise Rejected("SOURCE_COMMIT_MISMATCH")
    if git(source, "status", "--porcelain", "--untracked-files=normal"):
        raise Rejected("SOURCE_NOT_CLEAN")
    metadata = tomllib.loads((source / "pyproject.toml").read_text(encoding="utf-8"))
    if metadata["project"]["version"] != lock["product_version"]:
        raise Rejected("SOURCE_VERSION_MISMATCH")
    upstream = json.loads((source / "upstream.lock.json").read_bytes())
    if upstream["commit_sha"] != lock["contract_commit"]:
        raise Rejected("SOURCE_CONTRACT_MISMATCH")
    return source


def verify_schemas(lock: dict) -> Path:
    root = GSP_ROOT / "contracts" / lock["contract_tag"]
    manifest = (root / "HASH_MANIFEST.csv").read_bytes()
    if hashlib.sha256(manifest).hexdigest() != lock["fixture_manifest_sha256"]:
        raise Rejected("SCHEMA_MANIFEST_MISMATCH")
    count = 0
    for row in csv.DictReader(io.StringIO(manifest.decode("utf-8"))):
        prefix = "core/gsp/schemas/"
        if not row["path"].startswith(prefix):
            continue
        relative = Path(row["path"].removeprefix("core/gsp/"))
        if relative.is_absolute() or ".." in relative.parts:
            raise Rejected("SCHEMA_PATH_INVALID")
        path = root / relative
        if path.is_symlink() or not path.is_file():
            raise Rejected("SCHEMA_PATH_INVALID")
        if hashlib.sha256(path.read_bytes()).hexdigest() != row["sha256_in_upstream"]:
            raise Rejected("SCHEMA_BYTES_MISMATCH")
        if path.suffix == ".json":
            count += 1
    if count != lock["schema_artifact_count"]:
        raise Rejected("SCHEMA_COUNT_MISMATCH")
    return root / "schemas"


def reject_duplicate_keys(pairs: list[tuple]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise Rejected("DUPLICATE_JSON_KEY")
        result[key] = value
    return result


def run(source: Path, input_path: Path, output_path: Path) -> None:
    if os.path.lexists(output_path):
        raise Rejected("OUTPUT_EXISTS")
    if input_path.is_symlink() or not input_path.is_file():
        raise Rejected("INPUT_PATH_INVALID")
    with input_path.open("rb") as handle:
        raw = handle.read(MAX_INPUT_BYTES + 1)
    if len(raw) > MAX_INPUT_BYTES:
        raise Rejected("INPUT_LIMIT")
    corpus = json.loads(raw, object_pairs_hook=reject_duplicate_keys)
    if not isinstance(corpus, dict) or set(corpus) != {"manifest", "evidence_bundle"}:
        raise Rejected("CORPUS_SHAPE_INVALID")
    evidence = corpus["evidence_bundle"]
    query = evidence.get("normalized_query") if isinstance(evidence, dict) else None
    if not isinstance(query, str) or not query.strip():
        raise Rejected("QUERY_INVALID")
    lock = json.loads((GSP_ROOT / "upstream-lock.json").read_bytes())
    source = verify_source(source, lock)
    schemas = verify_schemas(lock)
    sys.path.insert(0, str(source / "src"))
    from gspgemma import GSPCompiler

    result = GSPCompiler(schema_root=schemas).compile_bundle(
        query=query, evidence_bundle=evidence
    )
    encoded = (json.dumps(result.as_dict(), sort_keys=True, allow_nan=False) + "\n").encode()
    if len(encoded) > MAX_OUTPUT_BYTES:
        raise Rejected("OUTPUT_LIMIT")
    # No file is published before successful compilation and validation. A partial
    # write remains untrusted: Ledger's validate-result must accept it before use.
    with output_path.open("xb") as handle:
        handle.write(encoded)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        run(args.source_root, args.input, args.output)
    except Rejected as error:
        print(f"GSP_SIDECAR_REJECTED:{error}", file=sys.stderr)
        return 2
    except Exception:
        # Never echo corpus contents, paths, environment values or upstream errors.
        print("GSP_SIDECAR_REJECTED:COMPILATION_FAILED", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
