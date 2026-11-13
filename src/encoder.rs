use crate::graph::Graph;

pub fn encode_ra(g: &Graph, message: &[u8]) -> Result<Vec<u8>, String> {
    if g.n <= g.m {
        return Err("RA matrix must have more columns than check rows".into());
    }
    let k = g.n - g.m;
    if message.len() != k {
        return Err(format!("message length is {}, expected {k}", message.len()));
    }
    // This encoder implements RA_encode.m's systematic double-diagonal accumulator.
    for i in 0..g.m {
        let parity_cols = g.row_edges[g.row_ptr[i]..g.row_ptr[i + 1]]
            .iter()
            .map(|&id| g.edge_col[id])
            .filter(|&col| col >= k)
            .collect::<Vec<_>>();
        let expected = if i == 0 {
            vec![k]
        } else {
            vec![k + i - 1, k + i]
        };
        if parity_cols.len() != expected.len()
            || expected.iter().any(|col| !parity_cols.contains(col))
        {
            return Err(format!(
                "row {i} does not have the RA double-diagonal parity structure"
            ));
        }
    }
    let mut codeword = vec![0; g.n];
    codeword[..k].copy_from_slice(message);
    let mut previous = 0;
    for i in 0..g.m {
        let info_parity = g.row_edges[g.row_ptr[i]..g.row_ptr[i + 1]]
            .iter()
            .filter_map(|&id| {
                let col = g.edge_col[id];
                (col < k && message[col] == 1).then_some(1)
            })
            .fold(0u8, |acc, bit| acc ^ bit);
        let parity = info_parity ^ previous;
        codeword[k + i] = parity;
        previous = parity;
    }
    Ok(codeword)
}
