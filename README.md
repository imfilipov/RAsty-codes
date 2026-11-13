# RA reconciliation — optimized Rust implementation

This is a standalone, modular Rust project for encoding and belief-propagation decoding of sparse binary Repeat-Accumulate (RA) / LDPC codes used in reverse reconciliation. It is an optimized implementation of the algorithms in the original [CV-QKD reconciliation repository](https://github.com/timothe-bramas/CV-QKD_reconciliation), with a command-line interface, parallel decoding, a Python wrapper, and Jupyter teaching notebooks.

The source is split into focused Rust modules: `graph.rs` stores sparse parity-check graphs, `encoder.rs` handles systematic RA encoding, `decoder.rs` implements sum-product BP and parallel node updates, `input.rs` validates bit input, and `cli.rs` defines the command line.

## Quick start

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

`decode(...)` returns a `DecodeResult` with the estimated bit string, syndrome status, iteration count, and elapsed time. `decode_cv(...)` accepts quadrature and soft-observation arrays. `decode_batch(...)` takes lists of frame strings and returns results in input order.

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
