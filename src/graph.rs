#[derive(Clone)]
pub struct Graph {
    pub(crate) m: usize,
    pub(crate) n: usize,
    pub(crate) row_ptr: Vec<usize>,
    pub(crate) row_edges: Vec<usize>,
    pub(crate) col_ptr: Vec<usize>,
    pub(crate) col_edges: Vec<usize>,
    pub(crate) edge_col: Vec<usize>,
}

impl Graph {
    pub fn from_edges(m: usize, n: usize, edges: &[(usize, usize)]) -> Result<Self, String> {
        if m == 0 || n == 0 {
            return Err("matrix dimensions must be positive".into());
        }
        let mut rows = vec![Vec::new(); m];
        let mut cols = vec![Vec::new(); n];
        let mut edge_col = vec![0; edges.len()];
        let mut seen = std::collections::HashSet::new();
        for &(r, c) in edges {
            if r >= m || c >= n {
                return Err(format!("edge ({r},{c}) lies outside {m}x{n} matrix"));
            }
            if !seen.insert((r, c)) {
                return Err(format!("duplicate edge ({r},{c})"));
            }
            let id = seen.len() - 1;
            rows[r].push(id);
            cols[c].push(id);
            edge_col[id] = c;
        }
        // IDs follow input order. Adjacency stores those IDs, avoiding a dense H or message matrix.
        let mut row_ptr = vec![0];
        let mut row_edges = Vec::with_capacity(edges.len());
        let mut col_ptr = vec![0];
        let mut col_edges = Vec::with_capacity(edges.len());
        for v in rows {
            row_edges.extend(v);
            row_ptr.push(row_edges.len());
        }
        for v in cols {
            col_edges.extend(v);
            col_ptr.push(col_edges.len());
        }
        Ok(Self {
            m,
            n,
            row_ptr,
            row_edges,
            col_ptr,
            col_edges,
            edge_col,
        })
    }

    /// Compute the binary parity-check syndrome `H * word (mod 2)`.
    pub fn syndrome(&self, word: &[u8]) -> Result<Vec<u8>, String> {
        if word.len() != self.n {
            return Err(format!(
                "word length is {}, expected {}",
                word.len(),
                self.n
            ));
        }
        Ok((0..self.m)
            .map(|row| {
                self.row_edges[self.row_ptr[row]..self.row_ptr[row + 1]]
                    .iter()
                    .fold(0, |acc, &edge| acc ^ word[self.edge_col[edge]])
            })
            .collect())
    }
}
