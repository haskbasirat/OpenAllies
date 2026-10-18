"""Synthetic integrity/negative tests; never stages a real release or runs a game."""
from pathlib import Path
import argparse
import hashlib
import json
import struct
import tempfile
import unittest
from unittest import mock
import zipfile

import package_development_09 as package
import check_development_09_package as checker


def pe():
    data=bytearray(256);data[:2]=b'MZ';struct.pack_into('<I',data,0x3c,64)
    data[64:68]=b'PE\0\0';struct.pack_into('<H',data,68,0x8664)
    struct.pack_into('<H',data,86,0x20);struct.pack_into('<H',data,88,0x20b)
    data[100:122]=b'PerMonitorV2 asInvoker\0'
    return bytes(data)


class Integrity(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
    def tearDown(self):self.temp.cleanup()
    def fixture(self):
        folder=self.root/'package';folder.mkdir()
        (folder/package.EXE_NAME).write_bytes(pe())
        (folder/'README.txt').write_text('Development diagnostic; original assets remain external.')
        (folder/'assets-cache').mkdir();(folder/'assets-cache/cache.bin').write_bytes(b'cached source')
        staged={'exe_sha256':package.sha(folder/package.EXE_NAME),
                'staged_files':[package.record(p,folder) for p in sorted(folder.rglob('*')) if p.is_file()]}
        (folder/'evidence').mkdir()
        (folder/'evidence/validation.json').write_text(json.dumps({'passed':True,'executable_sha256':staged['exe_sha256']}))
        manifest={'native_runtime_diagnostics_connected':True,'playable_runtime':False,'full_game_parity':False,
                  'standalone_game':False,'files':[package.record(p,folder) for p in sorted(folder.rglob('*')) if p.is_file()]}
        (folder/'manifest.json').write_text(json.dumps(manifest))
        self.archive(folder)
        return folder,staged
    def archive(self,folder,changed=None,extra=None):
        paths=[p for p in sorted(folder.rglob('*')) if p.is_file() and p.name!=package.ARCHIVE_NAME]
        with zipfile.ZipFile(folder/package.ARCHIVE_NAME,'w',zipfile.ZIP_DEFLATED) as archive:
            for path in paths:
                name=path.relative_to(folder).as_posix()
                archive.writestr(name,changed[name] if changed and name in changed else path.read_bytes())
            if extra:archive.writestr(extra,b'not approved')
    def test_pe_hash_architecture_and_truncation(self):
        p=self.root/'candidate.exe';p.write_bytes(pe())
        self.assertEqual(package.inspect_pe(p,package.sha(p))['architecture'],'AMD64 PE32+')
        with self.assertRaises(RuntimeError):package.inspect_pe(p,'0'*64)
        p.write_bytes(b'MZ')
        with self.assertRaises(RuntimeError):package.inspect_pe(p,package.sha(p))
    def test_reject_path_escape_and_windows_aliases(self):
        for name in ['../outside','/absolute','C:drive','a\\b','a//b','.','a/../b','']:
            with self.subTest(name=name),self.assertRaises(RuntimeError):package.relative_path(name)
    def test_valid_sealed_payload_and_crc(self):
        folder,staged=self.fixture();result=checker.check_sealed(folder,staged)
        self.assertTrue(result['all_payload_hashes_and_crc_verified'])
        self.assertEqual(result['payload_files'],5)
    def test_valid_crc_does_not_hide_changed_zip_payload(self):
        folder,staged=self.fixture();self.archive(folder,{'assets-cache/cache.bin':b'cached sourcf'})
        with self.assertRaisesRegex(RuntimeError,'SHA256 mismatch'):checker.check_sealed(folder,staged)
    def test_extra_zip_payload_is_rejected(self):
        folder,staged=self.fixture();self.archive(folder,extra='extra.txt')
        with self.assertRaisesRegex(RuntimeError,'ZIP payload differs'):checker.check_sealed(folder,staged)
    def test_staged_payload_change_is_rejected(self):
        folder,staged=self.fixture();(folder/'README.txt').write_text('changed')
        with self.assertRaisesRegex(RuntimeError,'Changed payload'):checker.check_sealed(folder,staged)
    def test_duplicate_case_path_is_rejected(self):
        p=self.root/'file';p.write_bytes(b'x')
        row=package.record(p,self.root)
        with self.assertRaisesRegex(RuntimeError,'case-colliding'):
            package.verified_rows(self.root,[row,{**row,'path':'FILE'}])
    def test_safe_launcher_keeps_map_parentheses_quoted_and_pauses(self):
        text=package.launcher_text(['--map-restore-check','DeveloperMaps/(3v3)Firefight'],True)
        self.assertIn('"DeveloperMaps/(3v3)Firefight"\npause\n',text)
        for value in ['%APPDATA%','name&command','quote"break','\ncommand']:
            with self.assertRaises(RuntimeError):package.launcher_text([value])
    def test_existing_release_cannot_be_staged(self):
        folder=self.root/'existing';folder.mkdir();sentinel=folder/'keep';sentinel.write_text('old')
        with mock.patch.object(package,'RELEASE',folder),self.assertRaisesRegex(RuntimeError,'already exists'):
            package.stage(argparse.Namespace())
        self.assertEqual(sentinel.read_text(),'old')
    def test_exit_zero_without_expected_json_is_failed(self):
        folder,staged=self.fixture();qa=self.root/'qa';spec=self.root/'spec.json'
        spec.write_text(json.dumps({'checks':[{'name':'native-load','arguments':['--map-restore-check','map'],
            'expected_exit_code':0,'json_equals':{'native_map_load_diagnostic':True}}]}))
        process=argparse.Namespace(stdout='{"native_map_load_diagnostic":false}',stderr='',returncode=0)
        with mock.patch.object(checker,'ROOT',self.root),mock.patch.object(checker,'QA',qa),mock.patch.object(checker.subprocess,'run',return_value=process):
            result=checker.run_checks(folder,staged,spec,qa/'smoke')
        self.assertFalse(result['passed']);self.assertIn('JSON assertion failed',result['checks'][0]['failure'])
        self.assertTrue((qa/'smoke/appdata').is_dir())


if __name__=='__main__':unittest.main()
