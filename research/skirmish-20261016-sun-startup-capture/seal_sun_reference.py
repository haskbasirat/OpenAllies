"""Seal actual startup Sun words; never overwrite the raw capture or old03."""
from pathlib import Path
import hashlib
import json
import os
import struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
NATIVE = GAME / 'reconstruction/native-game/runtime-expanded-v14'

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def save_new(p, value):
    with p.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')

def one(rows, label):
    found = [row for row in rows if row['label'] == label]
    assert len(found) == 1, (label, len(found))
    return found[0]

def image_seed(path):
    raw = path.read_bytes()
    pe = struct.unpack_from('<I', raw, 0x3c)[0]
    assert raw[pe:pe+4] == b'PE\0\0'
    count = struct.unpack_from('<H', raw, pe+6)[0]
    optsize = struct.unpack_from('<H', raw, pe+20)[0]
    base = struct.unpack_from('<I', raw, pe+24+28)[0]
    address = 0xb2814c
    rva = address-base
    for index in range(count):
        off = pe+24+optsize+40*index
        name = raw[off:off+8].rstrip(b'\0').decode()
        vsize, va, rawsize, rawoff = struct.unpack_from('<4I', raw, off+8)
        if va <= rva and rva+12 <= va+vsize:
            offset = rva-va
            assert offset+12 <= rawsize, 'Sun words are expected in initialized image bytes, not BSS'
            value = raw[rawoff+offset:rawoff+offset+12]
            return {'exe_sha256': sha(path), 'address': address, 'section': name,
                    'raw_file_offset': rawoff+offset, 'file_bytes': value.hex(),
                    'words': list(struct.unpack('<3I', value)),
                    'storage': 'initialized writable .data raw image bytes (not zero-fill tail)'}
    raise AssertionError('Sun span is not mapped by image sections')

def main():
    folder = HERE / 'capture-01'
    path = folder / 'capture.json'
    c = json.loads(path.read_bytes())
    oldpath = ROOT / 'research/development10/native-source-capture-03/capture.json'
    old = json.loads(oldpath.read_bytes())
    assert c['completed_source_return'] and c['completed_session_constructor']
    assert c['original_error'] is None and c['cleanup']['wait_result'] == 0
    assert c['cleanup']['exit_code'] == 0 and c['cleanup']['exit_process_event_observed']
    assert c['cleanup']['errors'] == []
    assert c['folder_redirection_observed'] and c['original_archives_preserved']
    assert c['source_exe_sha256'] == old['source_exe_sha256']
    assert c['runtime_pins'] == old['runtime_pins']
    assert c['original_archives_sha256'] == old['original_archives_sha256']
    assert c['script_sha256'] == sha(HERE / 'capture_sun_startup.py')
    assert c['watched_word_indices'] == [0, 1, 2]
    assert c['watched_addresses'] == [0xb2814c, 0xb28150, 0xb28154]
    assert len(c['watch_arms']) == 62 and c['observed_native_sun_writes'] == []
    for arm in c['watch_arms']:
        assert arm['addresses'] == c['watched_addresses']
        assert arm['requested_dr7'] & 0xffff00ff == arm['observed_dr7'] & 0xffff00ff
        for slot in range(3):
            assert arm['observed_dr7'] & (3 << (slot*2)) == 1 << (slot*2)
            assert (arm['observed_dr7'] >> (16+slot*4)) & 15 == 13
        assert arm['observed_dr7'] & (3 << 6) == 0
    assert c['observation_cutoff']['sun_words'] == [0, 0, 0]
    assert len(c['isolated_post_cutoff_watch_calibration_probes']) == 3
    for i, p in enumerate(c['isolated_post_cutoff_watch_calibration_probes']):
        assert p['index'] == i and p['dr6'] & 15 == 1 << i
        assert p['address'] == c['watched_addresses'][i]
        assert p['value'] == p['observed_bits'] == 0 and p['eip'] == 0x76fae7
    for row in c['global_captures']:
        assert row['sun_words'] == [0, 0, 0]
    sunentry = [row for row in c['global_captures'] if row['label'] == 'sun_ctor_entry']
    sunreturn = [row for row in c['global_captures'] if row['label'] == 'sun_ctor_return']
    assert not sunentry and not sunreturn, 'If a new capture observes Sun ctor, inspect it explicitly before changing scope'
    for label in ['source_entry', 'means_entry', 'values_entry', 'source_return']:
        a = one(c['captures'], label)
        b = one(old['captures'], label)
        for field in ['x87_control', 'x87_status', 'mxcsr', 'crt_sse2_global', 'globals', 'manager_means']:
            if field in b:
                assert a[field] == b[field], (label, field)
        assert {k:v['bits'] for k,v in a['settings'].items()} == {k:v['bits'] for k,v in b['settings'].items()}
    assert one(c['captures'], 'source_return')['actors'] == one(old['captures'], 'source_return')['actors']
    files = []
    for source in NATIVE.rglob('*'):
        if not source.is_file():
            continue
        relative = source.relative_to(NATIVE)
        if any(part.lower() in ['logs', 'userdata', 'saves', 'saved games', 'profiles'] for part in relative.parts):
            continue
        copied = folder / 'runtime' / relative
        assert copied.is_file() and not os.path.samefile(source, copied)
        digest = sha(source)
        assert digest == sha(copied), str(relative)
        files.append({'path': str(relative).replace('\\', '/'), 'source_sha256': digest, 'copy_sha256': digest})
    assert len(files) == 54
    manifest = folder / 'runtime-files.json'
    save_new(manifest, {'schema': 1, 'files': files})
    original_seed = image_seed(GAME / 'AA.exe')
    v14_seed = image_seed(NATIVE / 'AA.exe')
    assert original_seed['file_bytes'] == v14_seed['file_bytes'] == '00'*12
    out = HERE / 'sun-startup-reference.json'
    result = {
        'schema': 1, 'sun_address': 0xb2814c, 'words': c['observation_cutoff']['sun_words'],
        'source_identities_sha256': 'aa96125b653b33c0706e240b9bedec545698659a6f59181e9017e4b2e601632d',
        'runtime_pins': c['runtime_pins'], 'baseline_source_capture_sha256': sha(oldpath),
        'capture_sha256': sha(path), 'tracer_sha256': c['script_sha256'],
        'runtime_manifest_sha256': sha(manifest), 'image_seeds': [original_seed, v14_seed],
        'watched_addresses': c['watched_addresses'], 'armed_threads': len(c['watch_arms']),
        'native_writes_before_cutoff': 0, 'post_cutoff_calibration_probes': 3,
        'completed_source_return': True, 'completed_session_constructor': True,
        'observed_sun_constructor': False, 'cleanup_succeeded': True,
        'scope': 'Observed initialized image startup seed through native source completion and Session constructor; Sun constructor itself was not reached. Static Sun read-only reference/constructor proof belongs to the Sun implementation. No map, battle, or perpetual immutability claim.',
    }
    save_new(out, result)
    print(json.dumps({'reference_sha256': sha(out), 'capture_sha256': sha(path), 'runtime_manifest_sha256': sha(manifest), 'words': result['words']}))

if __name__ == '__main__':
    main()
