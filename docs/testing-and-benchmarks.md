# Testing, Octave equivalence, and performance

## Test layers

1. **Rust unit tests** cover bit parsing, systematic RA encoding, syndrome calculation, DV decoding, and deterministic outputs across thread counts.
2. **Python wrapper tests** exercise encode, decode, and batch APIs against the release executable.
3. **Octave equivalence tests** compare the full output estimate for small DV/CV/encoder fixtures and the 6,000-bit RA frame. The optional DVB-S2 test compares all 64,800 decoded bits as well.
4. **Notebook validation** executes every code cell headlessly to catch stale paths and wrapper/API errors.
5. **Performance benchmark** measures the original Octave DV implementation once and Rust medians over repeated runs, then measures four independent frames under batch parallelism.

GNU Octave is required only for reference/equivalence and baseline performance tests. Octave 11.3.0 was installed for this workspace at:

`C:\Program Files\GNU Octave\Octave-11.3.0\mingw64\bin\octave-cli.exe`

The PowerShell scripts auto-detect that path. If Octave is installed elsewhere, pass `-Octave <path-to-octave-cli.exe>`.

## Commands on Windows

```powershell
$cargo = 'C:\Users\244191\Documents\ChatGPT\ALSFRSR\ALSCARE-Rust\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe'
& $cargo test
& $cargo build --release
$env:PYTHONPATH = 'python'
python -m unittest discover -s python/tests -v
python scripts/validate_notebooks.py
.\scripts\test_octave_equivalence.ps1
.\scripts\test_octave_equivalence.ps1 -IncludeDvb
.\scripts\benchmark.ps1 -Runs 5
```

The Rust toolchain is installed but not configured as the global rustup default on this computer. Calling the full `cargo.exe` path works; `cargo fmt` through the rustup proxy does not. Use `rustfmt.exe --edition 2021 src\*.rs` from the same toolchain directory to format sources.

The DVB-S2 test may take several minutes because the original Octave reference allocates and updates dense sparse-matrix views. It is an exact implementation comparison, not a routine quick test.

## Performance interpretation

The optimized decoder represents only edges present in the sparse parity-check matrix, caches both row and column adjacency for the full decode, computes each check's aggregate once per iteration, and avoids starting nested parallel work inside each frame. `decode-batch` parses the matrix once and assigns independent frames to workers. This should improve large-frame or multi-frame throughput; very small matrices can be slower with many threads because thread startup and synchronization cost more than the work.

Benchmark values are machine-specific and should be reported with graph size, thread count, number of repeated runs, and convergence iterations. The scripts use fixed inputs and check successful decoding so that the timing cannot silently compare different workloads.

## Measurement recorded in this workspace

On the same fixed 6,000-bit RA frame, the Octave baseline took **23,227 ms** (one run). Release-mode Rust medians over three runs were **15.290 ms** with one worker, **13.598 ms** with two, **11.484 ms** with four, and **13.046 ms** with 16. That is about **1,519×** faster for Rust with one worker than this Octave run, and four workers were about **1.33×** faster than one Rust worker on this frame.

For four independent RA-6000 frames, Rust medians over three batch runs were **53.258 ms** with one frame worker, **36.028 ms** with two, and **19.398 ms** with four (about **2.75×** faster at four workers). Results include process and input-file overhead; use the same script and machine to compare later revisions. The number of workers is capped by available frames in batch mode.

