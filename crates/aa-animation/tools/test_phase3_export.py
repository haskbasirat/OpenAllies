"""Bounded negative fixtures for the newly supported constant channel reduction."""
import unittest
from types import SimpleNamespace as N
import export_families as export


def keys(values=(3.0, 3.0), forward=0.0, backward=0.0):
    return [N(time=float(i), value=v, forward=forward, backward=backward)
            for i, v in enumerate(values)]


class ConstantReduction(unittest.TestCase):
    def test_constant_quadratic_preserves_source_evidence(self):
        result = export.channel(keys(), 2)
        self.assertEqual(result['interpolation'], 1)
        self.assertEqual(result['source_interpolation'], 2)
        self.assertTrue(result['constant_reduction'])
        self.assertEqual([k['value'] for k in result['keys']], [3, 3])
        self.assertEqual(result['source_tangents'], [{'forward': 0, 'backward': 0}] * 2)

    def test_varying_value_tangent_and_tbc_are_rejected(self):
        for group, kind in [(keys((3, 4)), 2), (keys(forward=0.001), 2),
                            (keys(backward=-0.001), 2), (keys(), 3)]:
            with self.subTest(kind=kind, group=group), self.assertRaises(ValueError):
                export.channel(group, kind)

    def test_linear_is_not_reduced_or_retimed(self):
        result = export.channel(keys((1, 4)), 1)
        self.assertEqual(result, {'interpolation': 1,
                                 'keys': [{'time': 0, 'value': 1}, {'time': 1, 'value': 4}]})


if __name__ == '__main__':
    unittest.main()
