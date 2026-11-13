"""Thin, checked subprocess wrapper around the ``ra-reconcile`` executable.

Set ``RA_RECONCILE_BIN`` to use a specific binary or ``RA_CARGO`` to select
Cargo. If the release binary is absent, the wrapper builds it on first use.
"""

from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from typing import Iterable, Sequence


PROJECT_ROOT = Path(__file__).resolve().parents[2]


class ReconciliationError(RuntimeError):
    """Raised when the Rust CLI cannot parse input or execute a request."""


@dataclass(frozen=True)
class DecodeResult:
    estimate: str
    success: bool
    iterations: int
    elapsed_ms: float
    stdout: str


def _executable() -> Path:
    override = os.environ.get("RA_RECONCILE_BIN")
    if override:
        path = Path(override).expanduser().resolve()
        if not path.is_file():
            raise ReconciliationError(f"RA_RECONCILE_BIN does not exist: {path}")
        return path
    name = "ra-reconcile.exe" if os.name == "nt" else "ra-reconcile"
    candidate = PROJECT_ROOT / "target" / "release" / name
    if candidate.is_file():
        return candidate
    cargo = _cargo()
    result = subprocess.run(
        [str(cargo), "build", "--release", "--manifest-path", str(PROJECT_ROOT / "Cargo.toml")],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise ReconciliationError(
            "Could not build the Rust CLI. Install Rust or set RA_RECONCILE_BIN.\n"
            + result.stderr.strip()
        )
    if not candidate.is_file():
        raise ReconciliationError(f"Cargo completed but binary is missing: {candidate}")
    return candidate


def _cargo() -> Path:
    override = os.environ.get("RA_CARGO")
    if override:
        return Path(override).expanduser().resolve()
    found = shutil.which("cargo")
    if found:
        return Path(found)
    if os.name == "nt":
        home = Path.home()
        candidate = (
            home
            / "Documents"
            / "ChatGPT"
            / "ALSFRSR"
            / "ALSCARE-Rust"
            / ".rustup"
            / "toolchains"
            / "stable-x86_64-pc-windows-msvc"
            / "bin"
            / "cargo.exe"
        )
        if candidate.is_file():
            return candidate
    raise ReconciliationError("Cargo is not on PATH; set RA_CARGO to cargo.exe or RA_RECONCILE_BIN to a built executable")


def _run(args: Sequence[str], *, accepted=(0, 3)) -> str:
    command = [str(_executable()), *map(str, args)]
    result = subprocess.run(command, text=True, capture_output=True, check=False)
    if result.returncode not in accepted:
        raise ReconciliationError(
            f"Rust CLI exited with code {result.returncode}:\n{result.stderr.strip()}\n{result.stdout.strip()}"
        )
    return result.stdout


def _parse_result(stdout: str) -> DecodeResult:
    estimate = re.search(r"^estimate=(\d+)$", stdout, re.MULTILINE)
    success = re.search(r"^success=(true|false)$", stdout, re.MULTILINE)
    iterations = re.search(r"^iterations=(\d+)$", stdout, re.MULTILINE)
    elapsed = re.search(r"^elapsed_ms=([0-9.eE+-]+)$", stdout, re.MULTILINE)
    if not all((estimate, success, iterations, elapsed)):
        raise ReconciliationError(f"Could not parse decoder result:\n{stdout}")
    return DecodeResult(
        estimate=estimate.group(1),
        success=success.group(1) == "true",
        iterations=int(iterations.group(1)),
        elapsed_ms=float(elapsed.group(1)),
        stdout=stdout,
    )


def _matrix_path(matrix: str | os.PathLike[str]) -> str:
    path = Path(matrix).expanduser().resolve()
    if not path.is_file():
        raise FileNotFoundError(path)
    return str(path)


def decode(
    matrix: str | os.PathLike[str],
    alice: str,
    message: str,
    p: float,
    *,
    max_iter: int = 100,
    threads: int | None = None,
) -> DecodeResult:
    """Decode one discrete-variable frame.

    ``alice`` is Alice's noisy bit string, ``message`` is Bob's public
    ``C XOR B`` string, and ``p`` is the estimated A/B bit error probability.
    """
    args = ["decode", "--matrix", _matrix_path(matrix), "--alice", alice,
            "--message", message, "--p", p, "--max-iter", max_iter]
    if threads is not None:
        args += ["--threads", threads]
    return _parse_result(_run(args))


def encode(matrix: str | os.PathLike[str], message: str) -> str:
    """Systematically encode the k information bits and return n code bits."""
    stdout = _run(["encode", "--matrix", _matrix_path(matrix), "--message", message], accepted=(0,))
    match = re.search(r"^codeword=(\d+)$", stdout, re.MULTILINE)
    if not match:
        raise ReconciliationError(f"Could not parse encoded word:\n{stdout}")
    return match.group(1)


def decode_cv(
    matrix: str | os.PathLike[str],
    x: Sequence[float],
    soft: Sequence[float],
    snr_db: float,
    *,
    max_iter: int = 100,
    threads: int | None = None,
) -> DecodeResult:
    """Decode a CV frame from Alice's X quadratures and signed soft samples."""
    if len(x) != len(soft):
        raise ValueError("x and soft must have equal lengths")
    with tempfile.TemporaryDirectory(prefix="ra-reconcile-") as directory:
        x_path = Path(directory) / "x.txt"
        soft_path = Path(directory) / "soft.txt"
        x_path.write_text("\n".join(map(str, x)), encoding="utf-8")
        soft_path.write_text("\n".join(map(str, soft)), encoding="utf-8")
        args = ["decode-cv", "--matrix", _matrix_path(matrix), "--x", str(x_path),
                "--soft", str(soft_path), "--snr-db", snr_db, "--max-iter", max_iter]
        if threads is not None:
            args += ["--threads", threads]
        return _parse_result(_run(args))


def decode_batch(
    matrix: str | os.PathLike[str],
    alice_frames: Iterable[str],
    message_frames: Iterable[str],
    p: float,
    *,
    max_iter: int = 100,
    threads: int | None = None,
) -> list[DecodeResult]:
    """Decode independent DV frames concurrently; output preserves input order."""
    alice = list(alice_frames)
    messages = list(message_frames)
    if not alice or len(alice) != len(messages):
        raise ValueError("alice_frames and message_frames must have equal non-zero lengths")
    with tempfile.TemporaryDirectory(prefix="ra-reconcile-batch-") as directory:
        a_path, m_path = Path(directory) / "alice.txt", Path(directory) / "message.txt"
        a_path.write_text("\n".join(alice) + "\n", encoding="utf-8")
        m_path.write_text("\n".join(messages) + "\n", encoding="utf-8")
        args = ["decode-batch", "--matrix", _matrix_path(matrix), "--alice-file", str(a_path),
                "--message-file", str(m_path), "--p", p, "--max-iter", max_iter]
        if threads is not None:
            args += ["--threads", threads]
        stdout = _run(args)
    estimates = dict((int(i), bits) for i, bits in re.findall(r"^estimate_(\d+)=(\d+)$", stdout, re.MULTILINE))
    frames = [(int(i), ok == "true", int(it)) for i, ok, it in
              re.findall(r"^frame=(\d+) success=(true|false) iterations=(\d+)$", stdout, re.MULTILINE)]
    if len(frames) != len(alice) or len(estimates) != len(alice):
        raise ReconciliationError(f"Could not parse batch result:\n{stdout}")
    elapsed_match = re.search(r"^frames=\d+ all_success=(?:true|false) elapsed_ms=([0-9.eE+-]+)$", stdout, re.MULTILINE)
    elapsed_ms = float(elapsed_match.group(1)) / len(alice) if elapsed_match else float("nan")
    return [DecodeResult(estimates[i], ok, it, elapsed_ms, stdout) for i, ok, it in frames]

