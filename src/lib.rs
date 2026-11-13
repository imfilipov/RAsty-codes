//! Sparse RA-LDPC encoding and belief-propagation decoding.
pub mod cli;
pub mod decoder;
pub mod encoder;
pub mod graph;
pub mod input;

#[cfg(test)]
mod tests {
    use super::{
        decoder::{decode_dv, decode_llr},
        encoder::encode_ra,
        graph::Graph,
        input::parse_bits,
    };

    fn small_graph() -> Graph {
        Graph::from_edges(2, 4, &[(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (1, 3)]).unwrap()
    }

    #[test]
    fn systematic_encoder_satisfies_checks() {
        let g = small_graph();
        let c = encode_ra(&g, &[1, 0]).unwrap();
        assert_eq!(c, [1, 0, 1, 1]);
        assert_eq!(g.syndrome(&c).unwrap(), [0, 0]);
    }

    #[test]
    fn decoder_recovers_and_thread_count_is_deterministic() {
        let g = small_graph();
        let llr = [2.0, -2.0, -2.0, 2.0];
        let one = decode_llr(&g, &llr, 10, 1).unwrap();
        let two = decode_llr(&g, &llr, 10, 2).unwrap();
        assert!(one.1);
        assert_eq!(one, two);
        assert_eq!(one.0, [0, 1, 1, 0]);
        let dv = decode_dv(&g, &[0, 1, 1, 0], &[1, 0, 0, 0], 0.1, 10, 1).unwrap();
        assert!(dv.1);
        assert_eq!(dv.0, [0, 1, 1, 0]);
    }

    #[test]
    fn malformed_bits_are_rejected() {
        assert!(parse_bits("01x1", 4, "bits").is_err());
    }
}
