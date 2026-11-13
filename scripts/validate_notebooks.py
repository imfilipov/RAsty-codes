"""Execute notebook code cells headlessly to catch stale paths and API drift."""
from contextlib import redirect_stdout, redirect_stderr
import io
import json
import os
from pathlib import Path

os.environ.setdefault("MPLBACKEND", "Agg")
root = Path(__file__).resolve().parents[1]
for notebook in sorted((root / "notebooks").glob("*.ipynb")):
    namespace = {"__name__": "__main__"}
    cells = json.loads(notebook.read_text(encoding="utf-8"))["cells"]
    count = 0
    for index, cell in enumerate(cells, 1):
        if cell["cell_type"] != "code":
            continue
        source = "".join(cell["source"])
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            exec(compile(source, f"{notebook.name}:cell-{index}", "exec"), namespace)
        count += 1
    print(f"PASS {notebook.name}: executed {count} code cells")
