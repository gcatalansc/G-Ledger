"""Mechanical calibration on disposable synthetic Ledgers; not a user study.

Uses only the public CLI, an explicit agent identity and the qualified local
binary. Does not modify production code, existing Ledgers, or release assets.
Each run has a new directory and preserves all outputs, including failures.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4

EXPECTED_BINARY = "3d240fe74730f8942371f9d2cceeab62d001b6929414339bab455cec97a93438"
SOURCE_COMMIT = "53dc909fc34d2ca03f9ca73db3a1af3b54520610"
PROGRESS = (
    "D2 completada con regresión focalizada y tipos comprobados; se preservan los intentos fallidos "
    "y la evidencia del prototipo. La regresión global sigue pendiente. D3 requiere aprobación separada."
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--repository", type=Path, default=Path(__file__).resolve().parents[3])
    args = parser.parse_args()
    root, binary = args.repository.resolve(), args.binary.resolve()
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if digest != EXPECTED_BINARY:
        raise ValueError("Binary is not the pinned local qualification subject")
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid4().hex[:8]
    out = root / ".tooling/research/utility-v01" / run_id
    out.mkdir(parents=True, exist_ok=False)
    calls = []

    def cli(label: str, argv: list[str], expected: int = 0) -> str:
        started = time.perf_counter()
        result = subprocess.run([str(binary), *argv], capture_output=True, timeout=30)
        elapsed = (time.perf_counter() - started) * 1000
        number = len(calls) + 1
        stem = f"{number:03d}-{label}"
        (out / f"{stem}.stdout").write_bytes(result.stdout)
        (out / f"{stem}.stderr").write_bytes(result.stderr)
        calls.append({"label": label, "argv": argv, "exit_code": result.returncode,
                      "wall_ms": round(elapsed, 3), "stdout_bytes": len(result.stdout),
                      "stdout_sha256": hashlib.sha256(result.stdout).hexdigest()})
        (out / "calls.local.json").write_text(json.dumps(calls, ensure_ascii=False, indent=2), encoding="utf-8")
        if result.returncode != expected:
            raise RuntimeError(f"{label}: exit {result.returncode}; inspect local evidence in {out.name}")
        return result.stdout.decode("utf-8")

    resources = json.loads(cli("resource-verification", ["resources", "verify", "--json"]))
    initialized = cli("initialize", ["init", "--title", "Utility calibration — synthetic",
                                    "--summary", "Discardable research fixture; no real phase authority",
                                    "--timezone", "UTC", "--data-root", str(out / "data")])
    identity = dict(line.split("=", 1) for line in initialized.splitlines() if "=" in line)
    selection = ["--database", identity["database"], "--project-ref", identity["project_ref"]]
    state = json.loads(cli("initial-status", ["status", *selection, "--json"]))
    version = state["state_version"]
    write_number = 0

    def write(label: str, command: list[str], extra: list[str]) -> dict:
        nonlocal version, write_number
        write_number += 1
        value = json.loads(cli(label, [*command, *selection, "--expected-version", str(version),
                                      "--idempotency-key", f"research:{write_number}",
                                      "--actor-id", "agent:utility-research",
                                      "--observed-at", datetime.now(timezone.utc).isoformat(),
                                      *extra, "--json"]))
        new_state = json.loads(cli(label + "-status", ["status", *selection, "--json"]))
        if (value["status"] != "ACCEPTED" or new_state["state_version"] != value["new_state_version"]
                or new_state["state_version"] - version != len(value["emitted_event_refs"])):
            raise AssertionError("Accepted command result disagrees with observed event/version delta")
        version = new_state["state_version"]
        return value

    write("progress", ["record", "progress"], ["--summary", PROGRESS])
    denied = []
    for label, command, extra in [
        ("proposed-next", ["next", "set"], ["--title", "Solicitar decisión para D3", "--classification", "PROPOSED"]),
        ("open-blocker", ["blocker", "open"], ["--title", "Dependencia pendiente", "--severity", "HIGH"]),
    ]:
        result = json.loads(cli(label, [*command, *selection, "--expected-version", str(version),
                                       "--idempotency-key", f"research:denied:{label}",
                                       "--actor-id", "agent:utility-research", "--observed-at",
                                       datetime.now(timezone.utc).isoformat(), *extra, "--json"], expected=9))
        if result["status"] != "POLICY_DENIED" or result["new_state_version"] != version or result["emitted_event_refs"]:
            raise AssertionError("Denied agent operation advanced canonical state")
        denied.append({"operation": " ".join(command), "exit_code": 9, "status": result["status"], "event_delta": 0})
    write("checkpoint", ["checkpoint", "create"], ["--reason", "research-calibration"])
    observations = []
    for unknowns in [0, 1, 8]:
        while write_number < 2 + unknowns:
            ordinal = write_number - 1
            write(f"unknown-{ordinal}", ["unknown", "record"],
                  ["--subject", identity["project_ref"],
                   "--statement", f"Fixture sintético: falta evidencia de la dependencia {ordinal}",
                   "--criticality", "MATERIAL"])
        before = json.loads(cli(f"before-{unknowns}", ["status", *selection, "--json"]))
        timings = {"codex": [], "json": []}
        outputs = {}
        for repetition in range(5):
            # Counterbalance order; cache effects remain a limitation of local timing.
            formats = ["codex", "json"] if repetition % 2 == 0 else ["json", "codex"]
            for fmt in formats:
                outputs[fmt] = cli(f"resume-{unknowns}-{fmt}-{repetition}",
                                   ["resume", *selection, "--format", fmt])
                timings[fmt].append(calls[-1]["wall_ms"])
        after = json.loads(cli(f"after-{unknowns}", ["status", *selection, "--json"]))
        if (before["state_version"], before["state_digest"]) != (after["state_version"], after["state_digest"]):
            raise AssertionError("Read changed canonical identity")
        resume = json.loads(outputs["json"])
        refs = resume["critical_unknown_refs"]
        if len(refs) != unknowns:
            raise AssertionError("Structured resume omitted a critical unknown reference")
        unknown_line = next(line for line in outputs["codex"].splitlines() if line.startswith("- Critical unknowns:"))
        visible = unknown_line.removeprefix("- Critical unknowns:").strip()
        visible_count = 0 if visible == "-" else len(visible.split(","))
        observations.append({"critical_unknowns_recorded": unknowns, "critical_unknown_refs_json": len(refs),
                             "critical_unknown_refs_codex": visible_count, "codex_unknown_line": unknown_line,
                             "freshness": resume["checkpoint_freshness"],
                             "read_state_delta": after["state_version"] - before["state_version"],
                             "json_progress": resume["last_material_progress_summary"],
                             "full_progress_available_in_resume_json": PROGRESS == resume["last_material_progress_summary"],
                             "critical_tail_in_codex": "D3 requiere aprobación separada" in outputs["codex"],
                             "critical_tail_in_json": "D3 requiere aprobación separada" in outputs["json"],
                             "outputs_utf8_bytes": {fmt: len(value.encode("utf-8")) for fmt, value in outputs.items()},
                             "latency_ms": {fmt: {"samples": samples, "median": statistics.median(samples)}
                                            for fmt, samples in timings.items()}})
    cli("final-verify", ["verify", *selection, "--json"])
    cli("final-doctor", ["doctor", *selection, "--json"])
    summary = {"schema_version": "research-0.1", "run_id": run_id,
               "qualification": "MECHANICAL_CALIBRATION_ONLY", "binary_sha256": digest,
               "source_commit": SOURCE_COMMIT, "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               "resource_verification_exit": 0, "synthetic_fixture": True,
               "independent_participants": 0, "models_evaluated": 0, "tokens_measured": False,
               "human_time_measured": False, "declared_progress": PROGRESS,
               "accepted_mutations": write_number, "total_cli_calls": len(calls),
               "agent_used_as_human": False, "denied_agent_operations": denied, "observations": observations,
               "status_top_level_keys": sorted(state),
               "limitations": ["Single local machine, one synthetic Ledger, five reads per format per state.",
                               "Timings include process startup and caches; no comparative productivity claim.",
                               "Byte lengths are not tokenizer counts; preparing and maintaining facts is unmeasured."]}
    (out / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
