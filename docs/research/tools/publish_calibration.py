"""Collect already-produced, synthetic research evidence for local review.

"Publish" here only writes docs/research/results in this repository. There is
no network operation and no deployment. Raw argv/data stay under .tooling.
"""
import argparse
import hashlib
import json
from pathlib import Path

from continuity_brief import build_brief


def save_new_or_identical(path: Path, data: bytes) -> None:
    if path.exists():
        if path.read_bytes() != data:
            raise ValueError(f"Refusing to replace evidence: {path.name}")
    else:
        path.write_bytes(data)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_id")
    args = parser.parse_args()
    if Path(args.run_id).name != args.run_id or ".." in args.run_id:
        raise ValueError("Run ID must be a basename")
    root = Path(__file__).resolve().parents[3]
    run = root / ".tooling/research/utility-v01" / args.run_id
    summary_raw = (run / "summary.json").read_bytes()
    summary = json.loads(summary_raw)
    if not summary["synthetic_fixture"] or summary["qualification"] != "MECHANICAL_CALIBRATION_ONLY":
        raise ValueError("Only the synthetic calibration may be collected here")
    if summary["independent_participants"] != 0 or summary["models_evaluated"] != 0:
        raise ValueError("Participant evidence requires a separate privacy review")
    calls = json.loads((run / "calls.local.json").read_text(encoding="utf-8"))

    def output(label):
        matches = [(i, call) for i, call in enumerate(calls, 1) if call["label"] == label]
        if len(matches) != 1:
            raise ValueError(f"Ambiguous output: {label}")
        number, call = matches[0]
        data = (run / f"{number:03d}-{label}.stdout").read_bytes()
        if hashlib.sha256(data).hexdigest() != call["stdout_sha256"]:
            raise ValueError("Captured output drift")
        return data

    structured = output("resume-8-json-4")
    codex = output("resume-8-codex-4")
    value = json.loads(structured)
    digest = hashlib.sha256(structured).hexdigest()
    brief = build_brief(value, "resume-input.json")
    brief += f"\nSHA-256 de la entrada: `{digest}`.\n"
    verify = json.loads(output("final-verify"))
    doctor = json.loads(output("final-doctor"))
    if verify["integrity"] != "ok" or doctor["integrity"] != "ok":
        raise ValueError("Storage integrity not verified")
    for observed in summary["observations"]:
        if observed["read_state_delta"] != 0:
            raise ValueError("Read changed state")
    results = root / "docs/research/results"
    results.mkdir(exist_ok=True)
    files = {"calibration-C01.json": summary_raw, "resume-input.json": structured,
             "resume-codex.md": codex, "continuity-P1.md": brief.encode("utf-8"),
             "integrity-C01.json": (json.dumps({"verify": verify, "doctor": doctor}, indent=2) + "\n").encode("utf-8")}
    comparison = {"qualification": "PRESENTATION_PROPERTIES_ONLY", "source_run": args.run_id,
                  "critical_unknowns_source": len(value["critical_unknown_refs"]),
                  "critical_unknown_refs_codex": summary["observations"][-1]["critical_unknown_refs_codex"],
                  "p1_exposes_total_count": "Incógnitas críticas: 8" in brief,
                  "p1_labels_truncated_text": "Texto recortado" in brief,
                  "p1_recovers_missing_semantic_content": False,
                  "p1_assigns_authority": False, "input_unchanged": output("resume-8-json-4") == structured,
                  "p1_script_sha256": hashlib.sha256((Path(__file__).parent / "continuity_brief.py").read_bytes()).hexdigest(),
                  "files": {name: {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()} for name, data in files.items()},
                  "productivity_effect": None, "preferred_by_users": None,
                  "decision": "Keep as research candidate; no promotion to product on these measurements."}
    files["comparison-P1.json"] = (json.dumps(comparison, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    for name, data in files.items():
        save_new_or_identical(results / name, data)
    print(json.dumps(comparison, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
