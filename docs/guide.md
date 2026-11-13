# RA reconciliation: project guide and tutorial

## What this project does

Quantum key distribution (QKD) gives Alice and Bob correlated random bit strings. Noise makes some of their bits disagree. Before they can use a shared key, they run **information reconciliation**: one party sends a public message that helps the other correct errors. This repository implements the coding and decoding part for binary Repeat-Accumulate (RA) / low-density parity-check (LDPC) codes. It does not implement the complete QKD protocol or privacy amplification.

This Rust project is the optimized companion to the original [Octave/MATLAB CV-QKD reconciliation repository](https://github.com/timothe-bramas/CV-QKD_reconciliation). It keeps the same code and channel model while replacing dense message work with sparse graph storage, cached adjacency lists, and parallel updates.

## A small glossary

- **Bit**: a value in `{0,1}`.
- **XOR (`⊕`)**: addition modulo two: `0⊕0=0`, `0⊕1=1`, `1⊕0=1`, `1⊕1=0`.
- **Codeword**: the longer protected bit string produced from a shorter information string.
- **Parity check**: a rule requiring selected bits to XOR to zero.
- **Parity-check matrix `H`**: a binary matrix where `H[i,j]=1` means check `i` includes bit `j`.
- **Syndrome**: `s = H cᵀ mod 2`. A zero syndrome means every parity check passes. It does not prove that the decoded word is the transmitted word; a different valid codeword can also have zero syndrome.
- **LDPC**: low-density parity-check code. Its matrix `H` contains relatively few ones, so its checks form a sparse graph.
- **RA**: Repeat-Accumulate. Information bits are repeated into checks, then a simple accumulator generates parity bits.
- **Information length `k`**, **code length `n`**, **number of checks `m`**: for the systematic matrices here, `k=n−m`; code rate is `R=k/n`.
- **BSC bit error probability `p`**: probability that one binary symbol flips in a binary symmetric channel; this decoder expects `0<p<0.5`.
- **LLR**: log-likelihood ratio, a signed numeric measure of confidence in a bit. Positive values favor zero; negative values favor one.
- **BP**: belief propagation, the iterative exchange of LLR messages between bit nodes and parity-check nodes.
- **Frame**: one length-`n` codeword and its corresponding noisy/public observations.

## How reverse reconciliation is represented

Let `B` be Bob's measured bit string and `A` Alice's correlated but noisy string. Bob forms a codeword `C` from a random information key and sends

`M = C ⊕ B`

over the public classical channel. Alice computes

`A ⊕ M = C ⊕ (A ⊕ B)`.

If each bit of `A` differs from `B` with probability `p`, Alice sees the codeword through a BSC with error rate `p`. The decoder estimates `C`; the first `k` systematic bits are the candidate key. The public communication leaks information and must be accounted for in a complete QKD security analysis.

## RA structure and encoding equations

The parity-check matrix has systematic form `H=[A | T]`. The first `k` columns are information-bit connections; the final `m` columns form a lower double-diagonal accumulator:

```text
T = 1 0 0 0 ...
    1 1 0 0 ...
    0 1 1 0 ...
    0 0 1 1 ...
```

For information vector `u`, the parity vector `v` is computed row by row over GF(2):

`v₀ = ⊕_{j: A[0,j]=1} uⱼ`

`vᵢ = vᵢ₋₁ ⊕ (⊕_{j: A[i,j]=1} uⱼ)`, for `i>0`.

The codeword is `c=(u,v)`. This linear-time recurrence avoids solving a general dense system. The encoder validates the required double-diagonal structure before encoding.

## How belief propagation decodes

The graph has one variable node for each bit and one check node for each row of `H`. An edge represents a `1` in `H`. Each edge carries two messages: variable-to-check `q` and check-to-variable `r`.

For the observation `z=A⊕M`, the channel LLR is

`Lⱼ = (1−2zⱼ) ln((1−p)/p)`.

Equivalently, `Lⱼ=ln((1−p)/p)` for `zⱼ=0`, and its negative for `zⱼ=1`.

The variable node excludes the edge it is answering so it does not immediately echo the same evidence:

`qⱼ→ᵢ = Lⱼ + Σ_{i'∈N(j), i'≠i} rᵢ'→ⱼ`.

The sum-product check update combines all other incoming messages. Define

`φ(x) = −ln(tanh(x/2))`,

then

`rᵢ→ⱼ = (∏_{j'∈N(i), j'≠j} sign(qⱼ'→ᵢ)) · φ(Σ_{j'∈N(i), j'≠j} φ(|qⱼ'→ᵢ|))`.

After each round, a bit is estimated as `ĉⱼ=0` when `Lⱼ+Σᵢ rᵢ→ⱼ ≥ 0`, and as one otherwise. The decoder evaluates `H ĉᵀ mod 2`; it stops when the syndrome is zero or the iteration limit is reached. The implementation uses a total-sum/excluded-edge form of the check update so the per-check work is linear in its degree rather than recomputing every neighbor sum.

### CV soft inputs

For continuous-variable soft observations, the CLI uses the source model's equivalent LLR expression

`Lⱼ = 2 · 10^(SNR_dB/10) · Xⱼ · softⱼ`.

Here `X` is Alice's quadrature and `soft` is Bob's signed soft sample as prepared by the original channel model. `SNR_dB` is converted to a linear ratio with `10^(SNR_dB/10)`. Inputs are one finite non-zero `X` and one finite `soft` value per code bit.

## Worked example: two checks and four bits

The included `data/H_small.edges` matrix has `m=2`, `n=4`, and edges

```text
check 0: bits 0, 1, 2
check 1: bits 1, 2, 3
```

Its checks are `c₀⊕c₁⊕c₂=0` and `c₁⊕c₂⊕c₃=0`. There are `k=n−m=2` information bits. Encode `u=(1,0)`:

1. Row 0 contains information bit 0, so `v₀=1`.
2. Row 1 contains information bit 1 (zero) and the previous parity, so `v₁=1⊕0=1`.
3. Thus `c=(1,0,1,1)`. Both checks XOR to zero.

Run it:

```powershell
cargo run --release -- encode --matrix data/H_small.edges --message 10
# codeword=1011
```

For decoding, use `A=0110`, `M=1000`, and `p=0.1`. Alice's BSC observation is `A⊕M=1110`, which differs by one bit from the valid test word `0110`. BP returns that all-check-valid estimate for this fixed fixture:

```powershell
cargo run --release -- decode --matrix data/H_small.edges --alice 0110 --message 1000 --p 0.1 --threads 1
```

The example demonstrates syndrome decoding, not cryptographic security or a realistic operating point for a production QKD link.

## Sparse storage and parallel execution

`graph.rs` stores the edge list plus row and column compressed adjacency arrays. Memory and message updates therefore scale with the number of ones in `H`, not with `m×n`. The same adjacency is reused through all BP iterations. `--threads N` parallelizes independent rows during check updates and independent columns during variable updates. `decode-batch` processes independent frames on separate workers and shares one loaded graph. Deterministic ordering is retained when worker outputs are gathered.

For many short frames, use batch parallelism. For one large frame, node-level parallelism can help. Running both forms of parallelism at once is not supported: the batch worker count is the number of frame workers and each frame uses one internal worker, which avoids oversubscribing cores.

