"""Read original NIF headers without parsing objects or changing source files.

Original review utility. Uses only Python's standard library. Does not load or
execute SDK material. Its only write is the requested JSON evidence output.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import struct


def scan(root: Path) -> dict:
    paths = []
    for directory, _, files in os.walk(root):
        for filename in files:
            if filename.lower().endswith('.nif'):
                paths.append(Path(directory) / filename)
    paths.sort(key=lambda path: path.relative_to(root).as_posix().lower())
    records = []
    for path in paths:
        digest = hashlib.sha256()
        with path.open('rb') as source:
            header = source.readline(256)
            version = source.read(4)
            digest.update(header)
            digest.update(version)
            for block in iter(lambda: source.read(1024 * 1024), b''):
                digest.update(block)
        records.append({
            'path': path.relative_to(root).as_posix(),
            'bytes': path.stat().st_size,
            'sha256': digest.hexdigest(),
            'header': header.decode('ascii', errors='replace').rstrip('\r\n'),
            'version_after_header_le': (
                f'0x{struct.unpack("<I", version)[0]:08x}' if len(version) == 4 else None
            ),
        })
    return {
        'schema': 'aa-read-only-native-nif-header-census-v1',
        'source_root': str(root.resolve()),
        'method': 'Read header line and next four bytes; SHA-256 the complete original file. No SDK code or object parser runs.',
        'not_proven': 'Object-block layouts, custom stream loaders, runtime compatibility, rendering behavior, or original executable parity.',
        'count': len(records),
        'header_counts': dict(sorted(Counter(row['header'] for row in records).items())),
        'binary_version_counts': dict(sorted(Counter(row['version_after_header_le'] for row in records).items())),
        'records': records,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_root', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = scan(args.source_root)
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({key: result[key] for key in ('count', 'header_counts', 'binary_version_counts')}, indent=2))
