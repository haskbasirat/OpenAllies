"""Minimal reader of original RMP map archives (TGCK version 2): directory of
96-byte rows at the end {UTF-16 name[32], packed offset, packed bytes,
timestamp, decoded bytes}; payload zlib or raw (as aa-mapworld archive.rs).
Read only; used to feed real records to the native oracle."""
import struct, zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MAPS = ROOT / 'reconstruction/assets/original/Data/Maps'


class Rmp:
    def __init__(self, path):
        self.path = Path(path)
        b = self.b = self.path.read_bytes()
        assert b[:4] == b'TGCK' and struct.unpack_from('<I', b, 4)[0] == 2
        n = struct.unpack_from('<I', b, 8)[0]
        start = len(b) - 96 * n
        self.dir = {}
        for i in range(n):
            row = b[start + 96 * i:start + 96 * (i + 1)]
            name = row[:64].decode('utf-16-le').split('\0')[0]
            off, packed, ts, decoded = struct.unpack_from('<4Q', row, 64)
            self.dir[name] = (off, packed, decoded)

    def chunk(self, name):
        if name not in self.dir:
            return None
        off, packed, decoded = self.dir[name]
        raw = self.b[off:off + packed]
        try:
            d = zlib.decompress(raw)
            if len(d) == decoded:
                return d
        except zlib.error:
            pass
        assert len(raw) == decoded, name
        return raw


def index_lengths(index):
    version, count = struct.unpack_from('<II', index, 0)
    assert version == 0 and len(index) == 8 + 4 * count
    return list(struct.unpack_from('<%dI' % count, index, 8))


def records(data, lengths):
    """(start offset, length) of each record in a data chunk {version 0, header word, records}."""
    at = 8
    out = []
    for n in lengths:
        out.append((at, n))
        at += n
    assert at == len(data)
    return out
