"""Pin discovery evidence without changing any source repository.

Raw snapshots remain under .tooling; the shareable manifest contains only
repository-relative references, hashes, ranges and observations from the catalog.
Requires Python 3.11+ and git. No external packages, network or model calls.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=Path, default=Path(__file__).resolve().parents[3])
    args = parser.parse_args()
    root = args.repository.resolve()
    catalog_path = root / "docs/research/cases.v0.1.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    local = root / ".tooling/research/utility-v01/sources"
    local.mkdir(parents=True, exist_ok=True)
    manifest = {"schema_version": "research-0.1", "collected_at_utc": datetime.now(timezone.utc).isoformat(),
                "catalog_sha256": hashlib.sha256(catalog_path.read_bytes()).hexdigest(),
                "sources": {}, "cases": [], "qualification": "DOCUMENTARY_DISCOVERY_ONLY"}
    source_lines = {}
    for key, source in catalog["sources"].items():
        repo = (root.parent / source["repo"]).resolve()
        sha = catalog["pins"][source["repo"]]
        data = subprocess.run(["git", "-c", f"safe.directory={repo}", "-C", str(repo),
                               "show", f"{sha}:{source['path']}"], check=True, capture_output=True).stdout
        lines = data.decode("utf-8-sig").splitlines()
        digest = hashlib.sha256(data).hexdigest()
        snapshot = local / f"{key}-{digest}.txt"
        if snapshot.exists() and snapshot.read_bytes() != data:
            raise ValueError("Snapshot collision")
        snapshot.write_bytes(data)
        manifest["sources"][key] = dict(source, commit=sha, sha256=digest, bytes=len(data), lines=len(lines))
        source_lines[key] = lines
    for case in catalog["cases"]:
        lines = source_lines[case["source"]]
        matches = [i for i, line in enumerate(lines) if line.startswith(case["start"])]
        if len(matches) != 1:
            raise ValueError(f"{case['id']}: expected one start, found {len(matches)}")
        start = matches[0]
        end = next((i for i in range(start, len(lines)) if lines[i].startswith(case["end"])), None)
        if end is None:
            raise ValueError(f"{case['id']}: no end")
        selected = "\n".join(lines[start:end + 1]).encode("utf-8")
        manifest["cases"].append({"id": case["id"], "source": case["source"],
                                  "start_line": start + 1, "end_line": end + 1,
                                  "excerpt_lf_sha256": hashlib.sha256(selected).hexdigest()})
    destination = root / "docs/research/case-source-manifest.v0.1.json"
    destination.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"sources": len(manifest["sources"]), "cases": len(manifest["cases"]),
                      "projects": len(catalog["pins"]), "status": "PINNED"}))


if __name__ == "__main__":
    main()
