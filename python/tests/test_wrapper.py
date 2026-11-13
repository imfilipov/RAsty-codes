import unittest
from pathlib import Path

from ra_reconcile import decode, decode_batch, decode_cv, encode

ROOT = Path(__file__).resolve().parents[2]


class WrapperTests(unittest.TestCase):
    def test_encode_and_decode(self):
        matrix = ROOT / "data" / "H_small.edges"
        self.assertEqual(encode(matrix, "10"), "1011")
        result = decode(matrix, "0110", "1000", 0.1, max_iter=10, threads=1)
        self.assertEqual(result.estimate, "0110")
        self.assertTrue(result.success)

    def test_batch_keeps_frame_order(self):
        matrix = ROOT / "data" / "H_small.edges"
        results = decode_batch(matrix, ["0110", "0110"], ["1000", "1000"], 0.1,
                               max_iter=10, threads=2)
        self.assertEqual([result.estimate for result in results], ["0110", "0110"])
        self.assertTrue(all(result.success for result in results))

    def test_cv_wrapper(self):
        matrix = ROOT / "data" / "H_small.edges"
        result = decode_cv(matrix, [1.0] * 4, [-1.0, -1.0, -1.0, 1.0],
                           0.0, max_iter=10, threads=1)
        self.assertEqual(result.estimate, "0110")
        self.assertTrue(result.success)


if __name__ == "__main__":
    unittest.main()
