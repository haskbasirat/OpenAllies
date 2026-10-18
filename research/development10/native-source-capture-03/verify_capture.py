"""Read-only post-capture provenance checks; write only a new local report."""
from pathlib import Path
import hashlib
import json
import os

HERE = Path(__file__).resolve().parent
GAME = HERE.parents[3]
SOURCE = GAME / 'reconstruction/native-game/runtime-expanded-v14'


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    assert SOURCE.is_dir(), 'Original v14 source directory is missing'
    rows = []
    skipped = {'logs', 'userdata', 'saves', 'saved games', 'profiles'}
    for path in SOURCE.rglob('*'):
        if not path.is_file() or any(p.lower() in skipped for p in path.relative_to(SOURCE).parts):
            continue
        dest = HERE / 'runtime' / path.relative_to(SOURCE)
        rows.append({'path': path.relative_to(SOURCE).as_posix(), 'bytes': path.stat().st_size,
                     'source_sha256': sha(path), 'copy_sha256_after_capture': sha(dest),
                     'same_file': os.path.samefile(path, dest),
                     'copied_readonly_attribute': bool(dest.stat().st_file_attributes & 1)})
    report = {'schema': 1, 'scope': 'Post-capture source/copy equality and independent-file check; original archive hashes were also pinned before execution in capture.json',
              'source': str(SOURCE), 'records': rows,
              'all_copies_independent': all(not r['same_file'] for r in rows),
              'all_source_copy_bytes_equal_after_capture': all(r['source_sha256'] == r['copy_sha256_after_capture'] for r in rows),
              'all_rwd_copies_readonly': all(r['copied_readonly_attribute'] for r in rows if r['path'].lower().endswith('.rwd'))}
    assert rows, 'An empty source inventory cannot validate a capture'
    assert report['all_copies_independent'] and report['all_source_copy_bytes_equal_after_capture'] and report['all_rwd_copies_readonly']
    output = HERE / 'runtime-files.json'
    with output.open('x', encoding='utf-8') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps({'files': len(rows), 'sha256': sha(output), 'all_copies_independent': True,
                      'source_copy_bytes_equal_after_capture': True, 'rwd_copies_readonly': True}))


if __name__ == '__main__':
    main()
