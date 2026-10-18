"""Original chunk-file container ("TGCK") of saved games under Unicorn: the
framing around the METAGAME payload.

The save writer 4F2DB0 opens each chunk with 859150 (name, compress flag =
config variable "MapSaveCompressed", 41F9D0) on the stream S = ctx+3C and
writes the payload through S vtable +4; the loader 4FC600 opens chunks with
859030 (name, required) and reads through S vtable +0. Original code run here
(the base chunk file class, vtable AB4E38, ctor 8584A0):
  859690 open (877B80 keeps the file object, file vtable +24 open, 858540,
         859380 directory read / new-file header 859090),
  859150 open chunk for writing (descriptor 0x70: 8580E0 layout; previous
         chunk finished first; offset = previous end or 0x0C; hash 8585E0),
  858E40 write, 858DB0 chunk size update, 858C90 read, 858F50 chunk seek,
  859030 open chunk for reading (858590 lookup, case-insensitive),
  859330 close (859090: header {magic 'TGCK', version 2, count} at 0 and the
         0x60-byte directory records 8582B0 after the last chunk),
  858ED0 seek, 8581B0 tell, 8581C0 size, 858BA0 error (message
  "File: %s Chunk: %s Offset: %ld - " + reason, thrown).

SEAMS:
  * the engine file object at S+4 (file system): an in-memory file with
    vtable +00 delete, +18 is-open, +20 mode, +24 open, +28 close, +2C read,
    +30 write, +34 tell, +38 size, +3C seek(pos, origin), +40 flush;
  * the file spec given to 859690 (CString path at +0, mode at +10) is built
    here (856A30 is not run);
  * the CRT seams of native_save.py (format, realloc, throw) and of
    mg_world.install_crt (heap, __wcsicmp C locale).
Not run: compressed chunks (8B72E0.. zlib 1.1.4 deflate level 6, enabled by
MapSaveCompressed).
"""
import json, hashlib, random, struct, sys
from pathlib import Path
from unicorn.x86_const import UC_X86_REG_ECX
from mg_machine import Machine, P, V, VISITED, HERE
from mg_world import install_crt, install_wcsicmp, native_wstring
import native_save
from native_save import throw_seam, install_crt_format, install_crt_realloc, run_native

MAGIC = 0x4B434754
CHUNK_NAMES = ['RULES', 'TERRAIN', 'HEIGHT_MAP', 'MOUNTAIN_HEIGHT_MAP', 'IDMGR', 'RESTRICTION_GROUPS', 'TEAMS',
               'OBJECTIVES', 'KINGDOMS', 'METAGAME', 'USED_REQUIRED_OBJECTS_INDEX', 'USED_REQUIRED_OBJECTS',
               'DYNAMIC_THINGS_INDEX', 'DYNAMIC_THINGS', 'TRIGGERS', 'DENIZEN_GROUPS', 'SUN', 'UPDATABLES', 'MISC_DATA']


class FakeFile:
    """Engine file object (seam): an in-memory file."""

    def __init__(self, m, data=b'', mode=2):
        self.m = m
        self.data = bytearray(data)
        self.pos = 0
        self.mode = mode
        self.calls = []
        code = m.alloc(0x100)
        m.u.mem_write(code, b'\xc3' * 0x100)
        self.vt = m.alloc(0x100)
        slots = {0x00: self._delete, 0x18: self._is_open, 0x20: self._mode, 0x24: self._open, 0x28: self._close,
                 0x2c: self._read, 0x30: self._write, 0x34: self._tell, 0x38: self._size, 0x3c: self._seek,
                 0x40: self._flush}
        for off in range(0, 0x100, 4):
            m.w32(self.vt + off, code + off)
            fn = slots.get(off)
            m.seams[code + off] = fn if fn else self._unexpected(off)
        self.obj = m.alloc(0x20)
        m.w32(self.obj, self.vt)

    def _unexpected(self, off):
        def f(mm):
            raise RuntimeError(f'file vtable +{off:#x}')
        return f

    def _this(self, mm):
        assert mm.u.reg_read(UC_X86_REG_ECX) == self.obj

    def _delete(self, mm):
        self._this(mm); self.calls.append('delete'); mm.ret(4)

    def _is_open(self, mm):
        self._this(mm); mm.ret(0, 1)

    def _mode(self, mm):
        self._this(mm); mm.ret(0, self.mode)

    def _open(self, mm):
        self._this(mm); self.calls.append('open'); mm.ret(4, 1)

    def _close(self, mm):
        self._this(mm); self.calls.append('close'); mm.ret(0, 1)

    def _flush(self, mm):
        self._this(mm); self.calls.append('flush'); mm.ret(0, 1)

    def _read(self, mm):
        self._this(mm)
        dst, n = mm.arg(0), mm.arg(1)
        chunk = bytes(self.data[self.pos:self.pos + n])
        if len(chunk) < n:
            raise native_save.StreamEnd(f'file read {n} at {self.pos} of {len(self.data)}')
        mm.u.mem_write(dst, chunk)
        self.pos += n
        mm.ret(8, n)

    def _write(self, mm):
        self._this(mm)
        src, n = mm.arg(0), mm.arg(1)
        b = bytes(mm.u.mem_read(src, n))
        if self.pos > len(self.data):
            self.data += b'\0' * (self.pos - len(self.data))
        self.data[self.pos:self.pos + n] = b
        self.pos += n
        mm.ret(8, n)

    def _tell(self, mm):
        self._this(mm); mm.ret(0, self.pos)

    def _size(self, mm):
        self._this(mm); mm.ret(0, len(self.data))

    def _seek(self, mm):
        self._this(mm)
        pos, origin = mm.arg(0), mm.arg(1)
        assert origin == 0, origin
        self.pos = pos
        mm.ret(8, 1)


def machine():
    m = Machine(0x027f)
    install_crt(m)
    install_wcsicmp(m)
    m.seams[native_save.THROW] = throw_seam
    install_crt_format(m)
    install_crt_realloc(m)
    return m


def open_chunk_file(m, path, data, mode):
    """8584A0 (ctor) + 859690 (open) on a fake file object."""
    s = m.alloc(0x80)
    r = run_native(m, 0x8584a0, ecx=s)
    assert r[0] == 'ok', r
    f = FakeFile(m, data, mode)
    m.w32(s + 4, f.obj)
    spec = m.alloc(0x20)
    m.w32(spec, native_wstring(m, path))
    m.w32(spec + 0x10, mode)
    r = run_native(m, 0x859690, args=(spec,), ecx=s)
    return s, f, r


def wname(m, name):
    p = m.alloc(2 * len(name) + 2)
    m.u.mem_write(p, name.encode('utf-16le') + b'\0\0')
    return p


def write_file(m, path, chunks):
    """chunks: list of (name, [pieces]) written in order, then 859330 close."""
    s, f, r = open_chunk_file(m, path, b'', 2)
    assert r[0] == 'ok', r
    after_open = bytes(f.data)
    for name, pieces in chunks:
        r = run_native(m, 0x859150, args=(wname(m, name), 0), ecx=s)
        if r[0] != 'ok':
            return {'result': r[0], 'message': r[1]['message'] if r[0] == 'throw' else r[1]}, f
        for piece in pieces:
            buf = m.alloc(max(len(piece), 1))
            m.u.mem_write(buf, piece)
            r = run_native(m, m.r32(m.r32(s) + 4), args=(buf, len(piece)), ecx=s)
            assert r[0] == 'ok', r
    r = run_native(m, 0x859330, ecx=s)
    assert r[0] == 'ok', r
    return {'result': 'ok', 'after_open': after_open.hex(), 'file_calls': f.calls}, f


def read_file(m, path, data, requests):
    """Open for reading, then per request: 859030(name, required) and reads of
    the given sizes through S vtable +0."""
    s, f, r = open_chunk_file(m, path, data, 1)
    out = {'open': r[0]}
    if r[0] != 'ok':
        out['message'] = r[1]['message'] if r[0] == 'throw' else r[1]
        return out
    rows = []
    for name, required, sizes in requests:
        r = run_native(m, 0x859030, args=(wname(m, name), required), ecx=s)
        row = {'name': name, 'required': required}
        if r[0] != 'ok':
            row['result'] = r[0]
            row['message'] = r[1]['message'] if r[0] == 'throw' else r[1]
            rows.append(row)
            continue
        row['found'] = r[1] & 0xff
        reads = []
        if row['found']:
            for n in sizes:
                buf = m.alloc(max(n, 1))
                rr = run_native(m, m.r32(m.r32(s)), args=(buf, n), ecx=s)
                if rr[0] != 'ok':
                    reads.append({'size': n, 'result': rr[0], 'message': rr[1]['message'] if rr[0] == 'throw' else rr[1]})
                    break
                reads.append({'size': n, 'result': 'ok', 'returned': rr[1], 'bytes': bytes(m.u.mem_read(buf, n)).hex()})
        row['reads'] = reads
        rows.append(row)
    out['requests'] = rows
    return out


def directory(data):
    """Parse the written directory (version 2 records) for the report."""
    magic, version, count = struct.unpack_from('<III', data, 0)
    recs = []
    start = len(data) - count * 0x60
    for i in range(count):
        o = start + i * 0x60
        name = data[o:o + 0x40].decode('utf-16le').split('\0')[0]
        off, size, flags, usize = struct.unpack_from('<QQQQ', data, o + 0x40)
        recs.append({'name': name, 'offset': off, 'size': size, 'flags': flags, 'uncompressed_size': usize})
    return {'magic': magic, 'version': version, 'count': count, 'records': recs}


def payloads():
    """Real METAGAME payloads from native-save.json plus random bytes."""
    fx = json.loads((HERE / 'native-save.json').read_text())
    return [bytes.fromhex(c['rounds'][-1]['bytes']) for c in fx['campaigns']], hashlib.sha256((HERE / 'native-save.json').read_bytes()).hexdigest()


def main():
    rng = random.Random(0x859150)
    metagame, save_sha = payloads()
    files = []
    for case in range(24):
        m = machine()
        path = rng.choice(['Save\\campaign_slot_1.sav', 'C:\\Games\\AA\\Save\\Quick Save.sav', 'x.sav'])
        names = rng.sample(CHUNK_NAMES, rng.randint(0, len(CHUNK_NAMES)))
        if 'METAGAME' not in names and rng.random() < 0.7:
            names.insert(rng.randrange(len(names) + 1), 'METAGAME')
        if case == 1:
            names = []
        if case == 2:
            names = ['A' * 31, 'b', 'Mixed_Case_Chunk']
        chunks = []
        for name in names:
            if name == 'METAGAME':
                data = rng.choice(metagame)
            else:
                data = bytes(rng.getrandbits(8) for _ in range(rng.choice([0, 1, 4, 12, 100, rng.randint(0, 3000)])))
            cuts = sorted(rng.sample(range(1, len(data)), min(len(data) - 1, rng.randint(0, 4)))) if len(data) > 1 else []
            pieces = [data[a:b] for a, b in zip([0] + cuts, cuts + [len(data)])] if data else [b'']
            if rng.random() < 0.2:
                pieces = pieces + [b'']
            chunks.append((name, pieces))
        res, f = write_file(m, path, chunks)
        assert res['result'] == 'ok', res
        written = bytes(f.data)
        row = {'case': case, 'path': path, 'chunks': [[n, [p.hex() for p in ps]] for n, ps in chunks],
               'after_open': res['after_open'], 'file_calls': res['file_calls'], 'bytes': written.hex(),
               'directory': directory(written)}
        # read back natively: every chunk (case variants), whole and in pieces,
        # one missing chunk optional and required, one read past the end
        requests = []
        for name, pieces in chunks:
            total = sum(len(p) for p in pieces)
            variant = rng.choice([name, name.lower(), name.upper()])
            sizes = [total] if rng.random() < 0.5 else [len(p) for p in pieces if p]
            if rng.random() < 0.2:
                sizes = sizes + [1]
            requests.append((variant, rng.randint(0, 1), sizes))
        requests.append(('NO_SUCH_CHUNK', 0, []))
        if case % 3 == 0:
            requests.append(('NO_SUCH_CHUNK', 1, []))
        rng.shuffle(requests)
        m2 = machine()
        row['read'] = read_file(m2, path, written, [(n, r, s) for n, r, s in requests])
        row['read_requests'] = [[n, r, s] for n, r, s in requests]
        files.append(row)
    # reader cases on crafted inputs (synthetic inputs, native reader)
    base = bytes.fromhex(files[3]['bytes'])
    d3 = files[3]['directory']
    crafted = []
    def craft(name, data, requests, note):
        m = machine()
        crafted.append({'case': name, 'note': note, 'bytes': data.hex(), 'path': 'x.sav',
                        'read_requests': [[n, r, s] for n, r, s in requests],
                        'read': read_file(m, 'x.sav', data, requests)})
    first = d3['records'][0]['name'] if d3['records'] else 'RULES'
    craft('bad_magic', b'XXXX' + base[4:], [(first, 1, [])], '859380 magic check')
    craft('version_3', base[:4] + struct.pack('<I', 3) + base[8:], [(first, 1, [])], '859380 version > 2')
    count = d3['count']
    start = len(base) - count * 0x60
    def with_records(recs, version, rec_size):
        blob = bytearray(base[:start])
        blob[4:8] = struct.pack('<I', version)
        blob[8:12] = struct.pack('<I', len(recs))
        for r in recs:
            blob += r[:rec_size]
        return bytes(blob)
    recs = [base[start + i * 0x60:start + (i + 1) * 0x60] for i in range(count)]
    craft('version_1', with_records(recs, 1, 0x58), [(d3['records'][i]['name'], 1, [d3['records'][i]['size']]) for i in range(count)],
          'version 1: 0x58-byte records, uncompressed size = stored size')
    craft('version_0', with_records(recs, 0, 0x50), [(d3['records'][i]['name'], 1, [d3['records'][i]['size']]) for i in range(count)],
          'version 0: 0x50-byte records, no flags')
    if count >= 2:
        dup = bytearray(recs[1])
        dup[0:0x40] = recs[0][0:0x40].decode('utf-16le').lower().encode('utf-16le').ljust(0x40, b'\0')[:0x40]
        craft('duplicate_name', with_records([recs[0], bytes(dup)] + recs[2:], 2, 0x60), [(first, 1, [])],
              '859380: names hashed case-insensitively, duplicate -> error')
        empty = bytearray(recs[1]); empty[0:0x40] = b'\0' * 0x40
        craft('empty_name', with_records([recs[0], bytes(empty)] + recs[2:], 2, 0x60), [(first, 1, [])], '859380: empty name')
        far = bytearray(recs[1]); far[0x40:0x48] = struct.pack('<Q', len(base) + 1)
        craft('offset_past_end', with_records([recs[0], bytes(far)] + recs[2:], 2, 0x60), [(first, 1, [])], '859380: offset > size')
        big = bytearray(recs[0]); big[0x58:0x60] = struct.pack('<Q', 0)
        craft('logical_size_0', with_records([bytes(big)] + recs[1:], 2, 0x60), [(first, 1, [1])],
              '858C90: logical size (+0x58) checked for uncompressed chunks')
    craft('empty_file_read', b'', [('METAGAME', 0, [])], 'read mode on an empty file: 859380 reads the header')
    # write errors
    m = machine()
    res, _ = write_file(m, 'x.sav', [('', [b'abc'])])
    write_errors = [{'case': 'empty_chunk_name', 'result': res}]
    report = {
        'schema': 1, 'scope': __doc__.split('SEAMS:')[0].strip(), 'seams': __doc__.split('SEAMS:')[1].strip(),
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'save_fixture_sha256': save_sha,
        'files': files, 'crafted': crafted, 'write_errors': write_errors,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-chunks.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'files': len(files), 'crafted': len(crafted), 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
