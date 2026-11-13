use crate::graph::Graph;

pub fn dv_llr(alice: &[u8], message: &[u8], p: f64) -> Vec<f64> {
    let base = (p / (1.0 - p)).ln();
    (0..alice.len())
        .map(|j| {
            if (alice[j] ^ message[j]) == 1 {
                base
            } else {
                -base
            }
        })
        .collect()
}

fn phi(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else {
        -((x * 0.5).tanh()).ln()
    }
}
fn phi_inv(y: f64) -> f64 {
    if y <= 0.0 {
        0.0
    } else {
        let e = (-y).exp();
        ((1.0 + e) / (1.0 - e).max(f64::MIN_POSITIVE)).ln()
    }
}

// Deterministic parallel map. Each node's output is independent; results are reassembled in node order.
pub fn parallel_nodes<T: Send, F: Fn(usize) -> T + Sync>(
    count: usize,
    threads: usize,
    f: F,
) -> Vec<T> {
    if threads <= 1 || count <= 1 {
        return (0..count).map(f).collect();
    }
    let workers = threads.min(count);
    let chunk = (count + workers - 1) / workers;
    std::thread::scope(|scope| {
        let f = &f;
        let mut hs = Vec::new();
        for start in (0..count).step_by(chunk) {
            let end = (start + chunk).min(count);
            let ff = f;
            hs.push(scope.spawn(move || (start..end).map(ff).collect::<Vec<_>>()));
        }
        hs.into_iter()
            .flat_map(|h| h.join().expect("worker thread panicked"))
            .collect()
    })
}

/// Decode one binary-symmetric-channel frame from Alice's bits and the public message.
pub fn decode_dv(
    g: &Graph,
    alice: &[u8],
    msg: &[u8],
    p: f64,
    max_iter: usize,
    threads: usize,
) -> Result<(Vec<u8>, bool, usize), String> {
    if alice.len() != g.n || msg.len() != g.n {
        return Err("A and M must each have one bit per matrix column".into());
    }
    if !(0.0..0.5).contains(&p) {
        return Err("p must be strictly between 0 and 0.5".into());
    }
    let llr = dv_llr(alice, msg, p);
    decode_llr(g, &llr, max_iter, threads)
}
pub fn decode_llr(
    g: &Graph,
    llr: &[f64],
    max_iter: usize,
    threads: usize,
) -> Result<(Vec<u8>, bool, usize), String> {
    if llr.len() != g.n {
        return Err(format!("LLR length is {}, expected {}", llr.len(), g.n));
    }
    let e = g.row_edges.len();
    let mut q = vec![0.0; e];
    let mut r = vec![0.0; e];
    // First variable-to-check messages equal channel LLR, as r is initialized to zero.
    for j in 0..g.n {
        for ix in g.col_ptr[j]..g.col_ptr[j + 1] {
            q[g.col_edges[ix]] = llr[j];
        }
    }
    let mut estimate = vec![0; g.n];
    for iter in 1..=max_iter {
        // Check-to-variable update. Excluding one edge is O(1) after totals are computed.
        let rows = parallel_nodes(g.m, threads, |i| {
            let a = g.row_ptr[i];
            let b = g.row_ptr[i + 1];
            let mut sign = 1.0;
            let mut sum = 0.0;
            for &id in &g.row_edges[a..b] {
                let x = q[id];
                let ax = x.abs().max(1e-8);
                if x < 0.0 {
                    sign = -sign;
                }
                sum += phi(ax);
            }
            g.row_edges[a..b]
                .iter()
                .map(|&id| {
                    let x = q[id];
                    let sx = if x < 0.0 { -1.0 } else { 1.0 };
                    (sign * sx) * phi_inv((sum - phi(x.abs().max(1e-8))).max(0.0))
                })
                .collect::<Vec<_>>()
        });
        let mut ix = 0;
        for row in rows {
            for v in row {
                r[g.row_edges[ix]] = v;
                ix += 1;
            }
        }
        // Variable-to-check messages and posterior decisions.
        let cols = parallel_nodes(g.n, threads, |j| {
            let a = g.col_ptr[j];
            let b = g.col_ptr[j + 1];
            let total: f64 = g.col_edges[a..b].iter().map(|&id| r[id]).sum();
            let outgoing = g.col_edges[a..b]
                .iter()
                .map(|&id| llr[j] + total - r[id])
                .collect::<Vec<_>>();
            (outgoing, if llr[j] + total < 0.0 { 1 } else { 0 })
        });
        for (j, (out, bit)) in cols.into_iter().enumerate() {
            estimate[j] = bit;
            for (offset, v) in out.into_iter().enumerate() {
                q[g.col_edges[g.col_ptr[j] + offset]] = v;
            }
        }
        let syndrome_ok = (0..g.m).all(|i| {
            g.row_edges[g.row_ptr[i]..g.row_ptr[i + 1]]
                .iter()
                .fold(0u8, |s, &id| s ^ estimate[g.edge_col[id]])
                == 0
        });
        if syndrome_ok {
            return Ok((estimate, true, iter));
        }
    }
    Ok((estimate, false, max_iter))
}
