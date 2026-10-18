"""Meaningful original-source and non-overwrite checks for the new exporter."""
from __future__ import annotations
import json
from pathlib import Path
import struct
import tempfile
import unittest

import export_browser as export


class BrowserExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        material = json.loads(export.MATERIALS.read_text())
        cls.converter = export.Converter()
        cls.converter.audited_materials = material['models']
        cls.source = 'Buildings/GBR/GBRCorps/GBRCorpsDeath.nif'
        cls.data, cls.nodes, cls.meshes, cls.objects, _, _ = export.geometry.collect_model(
            export.ORIGINAL / cls.source, cls.converter)
        cls.clip = export.make_clip(cls.source, cls.data, cls.nodes, cls.objects, cls.meshes)

    def test_actual_hq_death_preserves_all_target_identities_and_curves(self):
        controllers = [c for c in self.data.blocks if isinstance(c, export.NifFormat.NiKeyframeController)]
        self.assertEqual(len(controllers), 33)
        self.assertEqual(len(self.clip['tracks']), len(controllers))
        objects = {id(obj): i for i, obj in enumerate(self.objects)}
        for controller, track in zip(controllers, self.clip['tracks']):
            self.assertEqual(track['node'], objects[id(controller.target)])
            self.assertEqual(track['target'], export.string(controller.target.name))
            self.assertEqual(track['flags'], int(controller.flags))
            self.assertEqual([k['time'] for k in track['translation']['keys']],
                             [float(k.time) for k in controller.data.translations.keys])
            rotation = track['rotation']
            self.assertEqual(rotation['kind'], 'euler_xyz')
            self.assertEqual(rotation['order'], struct.unpack('<I', struct.pack('<f', float(controller.data.unknown_float)))[0])
            for axis, channel in zip(controller.data.xyz_rotations, rotation['channels']):
                self.assertEqual(channel, export.channel(axis.keys, axis.interpolation))
            self.assertEqual(track['scale'], export.channel(controller.data.scales.keys, controller.data.scales.interpolation))
        self.assertGreater(self.clip['duration'], 6.63)
        self.assertLess(self.clip['duration'], 6.64)
        self.assertEqual(self.clip['unsupported_controllers'], [])

    def test_actual_hermite_tangents_are_not_reduced_or_scaled(self):
        axis = self.clip['tracks'][0]['rotation']['channels'][0]
        actual = [c for c in self.data.blocks if isinstance(c, export.NifFormat.NiKeyframeController)][0].data.xyz_rotations[0]
        self.assertEqual(axis['interpolation'], 2)
        self.assertGreater(len({k['value'] for k in axis['keys']}), 1)
        for key, raw in zip(axis['keys'], actual.keys):
            self.assertEqual(key['forward'], float(raw.forward))
            self.assertEqual(key['backward'], float(raw.backward))

    def test_external_kf_missing_targets_rejects_whole_binding(self):
        source = 'Shared/Animations/Units/Infantry/Idle.kf'
        data = export.base.read_nif(export.ORIGINAL / source)
        sequence = next(x for x in data.roots if isinstance(x, export.NifFormat.NiControllerSequence))
        with self.assertRaisesRegex(ValueError, 'KF target unresolved or ambiguous'):
            export.make_clip(source, data, self.nodes, self.objects, self.meshes, sequence)

    def test_unsupported_particle_effects_remain_explicit(self):
        source = 'Buildings/USA/USACorps/USACorpsDeath.nif'
        data, nodes, meshes, objects, _, _ = export.geometry.collect_model(export.ORIGINAL / source, self.converter)
        clip = export.make_clip(source, data, nodes, objects, meshes)
        self.assertEqual(len(clip['tracks']), 24)
        classes = {c['controller'] for c in clip['unsupported_controllers']}
        self.assertEqual(classes, {'NiPSysEmitterCtlr', 'NiPSysUpdateCtlr', 'NiAlphaController'})
        self.assertTrue(any('no retained triangle mesh' in c['reason'] for c in clip['unsupported_controllers']))

    def test_staging_write_refuses_overwrite(self):
        with tempfile.TemporaryDirectory(dir=export.HERE, prefix='test-exclusive-') as folder:
            path = Path(folder) / 'rig.json'
            export.write_new(path, {'source': 'first'})
            before = path.read_bytes()
            with self.assertRaises(FileExistsError):
                export.write_new(path, {'source': 'second'})
            self.assertEqual(path.read_bytes(), before)

    def test_actual_tcb_nested_serialized_tuple_is_preserved(self):
        source = export.ORIGINAL/'Units/JPN/JPNBattleship/JPNBattleshipRecoil1.kf'
        data = export.base.read_nif(source)
        controller = next(c for c in data.blocks if isinstance(c, export.NifFormat.NiKeyframeController)
                          and int(c.data.translations.interpolation) == 3)
        raw = list(controller.data.translations.keys)
        result = export.channel(raw, 3)
        for key, actual in zip(result['keys'], raw):
            self.assertEqual((key['tension'], key['bias'], key['continuity']),
                             (float(actual.tbc.t), float(actual.tbc.b), float(actual.tbc.c)))


if __name__ == '__main__':
    unittest.main(verbosity=2)
