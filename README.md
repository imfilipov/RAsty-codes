# RA reconciliation — optimized Rust implementation

This is a standalone, modular Rust project for encoding and belief-propagation decoding of sparse binary Repeat-Accumulate (RA) / LDPC codes used in reverse reconciliation. It is an optimized implementation of the algorithms in the original [CV-QKD reconciliation repository](https://github.com/timothe-bramas/CV-QKD_reconciliation), with a command-line interface, parallel decoding, a Python wrapper, and Jupyter teaching notebooks.

The source is split into focused Rust modules: `graph.rs` stores sparse parity-check graphs, `encoder.rs` handles systematic RA encoding, `decoder.rs` implements sum-product BP and parallel node updates, `input.rs` validates bit input, and `cli.rs` defines the command line.

## Quick start

### Windows PowerShell

```powershell
cargo test
cargo build --release
target\release\ra-reconcile.exe --help
```

On Windows, use the `.exe` suffix. The Python wrapper can build the release binary automatically on first use. To select an explicit binary or Cargo executable, set `RA_RECONCILE_BIN` or `RA_CARGO`.

```powershell
$env:PYTHONPATH = "python"
python -c "from ra_reconcile import encode, decode; h='data/H_small.edges'; print(encode(h, '10')); print(decode(h, '0110', '1000', 0.1))"
```

### Linux and macOS (POSIX shell)

Install the Rust stable toolchain and Python 3, then run these commands from the project root:

```sh
cargo test
cargo build --release
./target/release/ra-reconcile --help
PYTHONPATH=python python3 -c 'from ra_reconcile import encode, decode; h="data/H_small.edges"; print(encode(h, "10")); print(decode(h, "0110", "1000", 0.1))'
```

The Python wrapper builds `target/release/ra-reconcile` automatically if it is missing. Set `RA_RECONCILE_BIN` or `RA_CARGO` to use a non-default binary or Cargo path. To use the notebooks, install their dependencies with `python3 -m pip install -r requirements-notebooks.txt`, then run `jupyter lab notebooks`.

`decode(...)` returns a `DecodeResult` with the estimated bit string, syndrome status, iteration count, and elapsed time. `decode_cv(...)` accepts quadrature and soft-observation arrays. `decode_batch(...)` takes lists of frame strings and returns results in input order.

## Python Wrapper

The `ra_reconcile` package is a thin Python interface to the Rust CLI. It uses only the Python standard library; NumPy is not required by the wrapper. From the project root, add the package directory to Python's import path:

```powershell
# Windows PowerShell
$env:PYTHONPATH = "python"
```

```sh
# Linux/macOS
export PYTHONPATH=python
```

If the release executable is not present under `target/release`, the first API call builds it with `cargo build --release`. Cargo must be available on `PATH`, or set `RA_CARGO` to its executable. To use an already-built executable, set `RA_RECONCILE_BIN` to its full path. The matrix argument is a path to the sparse edge-list file: its first line contains `m n`, followed by zero-based check-node/bit-node pairs.

### Systematic RA encoding

`encode(matrix, message)` takes a string of `k=n−m` information bits and returns the `n`-bit systematic codeword:

```python
from ra_reconcile import encode

codeword = encode("data/H_small.edges", "10")
print(codeword)  # 1011
```

### Discrete-variable decoding

`decode(matrix, alice, message, p, *, max_iter=100, threads=None)` accepts Alice's length-`n` noisy bit string, Bob's public string `M=C XOR B`, and the estimated bit-error probability `p` (`0 < p < 0.5`). `max_iter` limits belief-propagation iterations. `threads` controls parallel check/bit-node updates for this frame; when omitted, Rust uses the available CPU parallelism.

```python
from ra_reconcile import decode

result = decode(
    "data/H_small.edges",
    alice="0110",
    message="1000",
    p=0.1,
    max_iter=10,
    threads=1,
)
print(result.estimate)    # 0110
print(result.success)     # True when the parity-check syndrome is zero
print(result.iterations, result.elapsed_ms)
```

### Continuous-variable soft decoding

`decode_cv(matrix, x, soft, snr_db, *, max_iter=100, threads=None)` accepts one quadrature `x` and one signed soft observation `soft` per code bit. `snr_db` is the signal-to-noise ratio in decibels. The wrapper writes these arrays to temporary files for the CLI and removes the files after decoding.

```python
from ra_reconcile import decode_cv

result = decode_cv(
    "data/H_small.edges",
    x=[1.0, 1.0, 1.0, 1.0],
    soft=[-1.0, -1.0, -1.0, 1.0],
    snr_db=0.0,
    max_iter=10,
    threads=1,
)
```

### Parallel batch decoding

`decode_batch(matrix, alice_frames, message_frames, p, *, max_iter=100, threads=None)` accepts equal-length, non-empty iterables of frame strings. Independent frames are decoded concurrently, the parity-check graph is loaded once, and results remain in input order. Here `threads` is the number of frame workers; each frame uses one internal worker to avoid nested oversubscription.

```python
from ra_reconcile import decode_batch

results = decode_batch(
    "data/H_small.edges",
    alice_frames=["0110", "0110"],
    message_frames=["1000", "1000"],
    p=0.1,
    max_iter=10,
    threads=2,
)
for frame_number, result in enumerate(results, start=1):
    print(frame_number, result.estimate, result.success, result.iterations)
```

### Results and errors

Single-frame decode calls return a `DecodeResult` with:

- `estimate`: the estimated length-`n` codeword as a string of `0` and `1`.
- `success`: whether the decoded word has zero syndrome. This does not guarantee it is the transmitted codeword.
- `iterations`: the number of BP iterations performed.
- `elapsed_ms`: elapsed CLI time in milliseconds. In batch results this is the batch elapsed time divided by the frame count.
- `stdout`: the CLI's textual summary.

The wrapper raises `ReconciliationError` for CLI/input/build failures. Reaching the iteration limit without a zero syndrome is returned as a result with `success=False`, so callers can inspect or retry it.

## Commands

```text
ra-reconcile encode --matrix data/H_small.edges --message 10
ra-reconcile decode --matrix data/H_small.edges --alice 0110 --message 1000 --p 0.1
ra-reconcile decode-cv --matrix data/H_small.edges --x x.txt --soft soft.txt --snr-db 0
ra-reconcile decode-batch --matrix data/RA_n6000_R150.edges --alice-file A.txt --message-file M.txt --p 0.01 --threads 4
```

Run `ra-reconcile --help` for full parameter definitions, file formats, and exit codes.

## Documentation and demonstrations

- [`docs/guide.md`](docs/guide.md): project purpose, RA and LDPC terms, equations, and a worked 2-check/4-bit example.
- [`notebooks/`](notebooks/): beginner tutorial, parameter study, and reproducible Monte Carlo simulations.
- [`docs/testing-and-benchmarks.md`](docs/testing-and-benchmarks.md): Octave equivalence procedure and performance benchmarks.
- [`python/ra_reconcile/`](python/ra_reconcile/): typed Python API backed by the Rust binary.

Open the notebooks from the project root with `jupyter lab notebooks`. They use the included edge-list matrices. Install the notebook environment with `python -m pip install -r requirements-notebooks.txt`. The wrapper itself uses only the Python standard library.

## Performance and parallelism

The sparse graph is loaded once per process. The decoder stores one message per graph edge rather than allocating dense check-by-bit matrices, caches row/column adjacency, and parallelizes independent node updates. In batch mode, independent frames are processed concurrently and share the already-loaded graph. Use `--threads` to control worker count; more threads help large graphs or frame batches, while small frames may be faster with one worker.

Timings depend on machine, compiler, graph, and convergence iterations. The benchmark scripts report measurements for the actual machine; no fixed speedup is promised for every workload.

## License and attribution

The upstream algorithms and original MATLAB/Octave files remain attributed to their authors. Consult the upstream repository for its source-file license and attribution terms before redistributing adapted files.
