"""P1: offline experimental view over resume --format json.

It exposes counts and limits, not inferred facts or authorization. It never
reads a Ledger database, invokes a model, writes canonical events or summarizes
historical material. The source JSON must accompany any shared view.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

COUNTS = [
    ("active_phase_refs", "Fases activas"),
    ("completed_phase_refs", "Fases completadas"),
    ("current_decision_refs", "Decisiones vigentes"),
    ("open_blocker_refs", "Bloqueos"),
    ("open_risk_refs", "Riesgos"),
    ("pending_gate_refs", "Gates pendientes"),
    ("critical_unknown_refs", "Incógnitas críticas"),
    ("conflict_refs", "Conflictos"),
]


def build_brief(value: dict, source_name: str) -> str:
    if value.get("schema_version") != "0.2.0":
        raise ValueError("Unsupported source schema")
    if not isinstance(value.get("state_version"), int) or isinstance(value.get("state_version"), bool):
        raise ValueError("Missing state version")
    for field in ["project_ref", "state_digest", "checkpoint_freshness"]:
        if not isinstance(value.get(field), str) or "\n" in value[field] or "`" in value[field]:
            raise ValueError(f"Invalid identity field: {field}")
    for field, _ in COUNTS:
        refs = value.get(field)
        if not isinstance(refs, list) or any(not isinstance(ref, str) for ref in refs):
            raise ValueError(f"Invalid reference list: {field}")
        if len(set(refs)) != len(refs):
            raise ValueError(f"Duplicate reference: {field}")
    if Path(source_name).name != source_name or any(char in source_name for char in "\n\r`[]()"):
        raise ValueError("Source must have a simple local basename")
    progress = value.get("last_material_progress_summary")
    if progress is not None and not isinstance(progress, str):
        raise ValueError("Invalid progress summary")
    action = value.get("preferred_next_action_ref")
    if action is not None and (not isinstance(action, str) or "\n" in action or "`" in action):
        raise ValueError("Invalid action reference")
    lines = ["# Continuidad — prototipo P1", "", "Vista experimental derivada; no es una decisión ni una verificación nueva.", "",
             f"Proyecto: `{value['project_ref']}`. Estado: **v{value['state_version']}**.",
             f"Digest del estado: `{value['state_digest']}`.",
             f"Checkpoint: `{value['checkpoint_freshness']}`; no verifica el checkout externo.", ""]
    lines.extend(f"- {label}: {len(value[field])}" for field, label in COUNTS)
    lines += ["", f"Referencias completas: [{source_name}]({source_name}). Los conteos incluyen todas las referencias recibidas; sus contenidos requieren consulta.", ""]
    if progress:
        # Plain lines are blockquoted to preserve their status as supplied data.
        lines.extend("> " + line for line in progress.splitlines())
        if progress.endswith("…"):
            lines += ["", "Texto recortado: recuperar el registro original antes de concluir su alcance."]
    else:
        lines.append("Avance: sin resumen en esta entrada; no implica ausencia de trabajo.")
    lines += ["", (f"Próxima acción: `{action}`; clasificación y precondiciones requieren consulta."
                      if action else "Próxima acción: sin registro; no establece autorización."), ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    body = build_brief(json.loads(raw.decode("utf-8")), args.input.name)
    body += "\nSHA-256 de la entrada: `" + hashlib.sha256(raw).hexdigest() + "`.\n"
    if args.output:
        # A new filename is required; earlier evidence is never silently replaced.
        with args.output.open("x", encoding="utf-8", newline="\n") as output:
            output.write(body)
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        print(body, end="")


if __name__ == "__main__":
    main()
