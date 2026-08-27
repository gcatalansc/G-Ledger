"""Test-only synchronization at actual GSP compilation entry; never an adapter option."""

from pathlib import Path
import runpy
import sys
import time

marker = Path(sys.argv[1])
adapter = sys.argv[2]
sys.argv = sys.argv[2:]


def observe_compile(frame, event, _arg):
    if (
        event == "call"
        and frame.f_code.co_name == "compile_evidence"
        and frame.f_globals.get("__name__") == "gspgemma.compiler.pipeline"
    ):
        sys.setprofile(None)
        staging = marker.with_name(marker.name + ".staging")
        if marker.exists():
            raise RuntimeError("fault marker already exists")
        with staging.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write("GSP_COMPILE_ENTERED\n")
        staging.rename(marker)
        # The owning Rust test kills and reaps this exact child. If the owner
        # fails, the test probe expires rather than leaving an orphan waiting.
        time.sleep(30)
        raise RuntimeError("fault handshake expired without process termination")


sys.setprofile(observe_compile)
runpy.run_path(adapter, run_name="__main__")
