"""Oracle: ORIGINAL map string-table loading (KKC_Data::LoadMapStrings 47EA40).

For every original map the MAPSTRINGS chunk bytes (and synthetic variants)
are fed to the original data-file resource: 473050 reader, 5BD2C0 load (5AA990
memory stdio-file, 856F90 UTF-8 character reader, 857190/856D20 line reader,
5BD080 TGC_DataFileParser with its 5BCC80/5BEFD0 state tables, 494000
DisplayString escapes), the "Text" sub-node (5B4410) is loaded into a
KKC_TextData (475180 ctor, vtable[0] -> 4BA710 LoadData) and registered into
a KKC_TextDataManager (4BA2A0, DataCheckingStrict = true as in UVars.tgi).
Every key is then resolved with 4B9D40 (lookup) and 4B9F40 GetDisplayString.

Seams: heap, CString manager, pools, logging, timers, the parser lock,
Win32 imports, game variables (LanguageIDS = English, DataCheckingStrict),
and MultiByteToWideChar (CP_UTF8, one byte at a time, Python model).
A C++ throw (96BEF5) ends the case and is recorded as a native error.
Writes native-mapstrings.json.
"""
from pathlib import Path
import hashlib, json, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parents[1] / 'research/phase7-triggers'))
import loader as L  # noqa: E402
import decode_census as dc  # noqa: E402

DATA = HERE.parents[2] / 'reconstruction/assets/original/Data/Maps'


class Thrown(Exception):
    pass


class MapStrings(L.Loader):
    def __init__(self):
        super().__init__()
        self.call(0x5bcc80, 0xb3db80)           # TGC_ParserFunctions table
        self.call(0x5befd0, 0xb3dbdc)           # tokenizer table
        lock = self.alloc(0x40)
        vt = self.alloc(0x40)
        for i in range(16):
            self.w(vt + i * 4, L.STUBS + 0x300 + i * 0x10)
            self.seam(L.STUBS + 0x300 + i * 0x10, lambda: self.ret(0))
        self.w(lock + 4, vt)
        self.w(0xb39bc8, lock)
        self.seam(0x59aba0, lambda: self.ret(0, 0))
        resmgr = self.alloc(0x200)
        base = self.alloc(0x20)
        self.w(base + 4, self.cstring(''))
        self.w(resmgr + 0x90, base)
        self.w(0xb36540, resmgr)
        avars = self.r(0xb39990)
        for off, value in [(0x80, 0x26), (0x84, 0x2e), (0x90, 0x5c)]:
            cell = self.alloc(4)
            self.w(cell, value)
            self.w(avars + off, cell)
        gamedata = self.alloc(0x1000)
        escapes = self.alloc(0x40)
        self.w(escapes + 0x28, 1)
        self.w(gamedata + 0x3bc, escapes)
        self.w(0xb36530, gamedata)
        self.vars['datacheckingstrict'] = ('byte', 1)
        self.w(0xb3647c, L.STUBS + 0x600)
        self.seam(L.STUBS + 0x600, self._mbtowc)
        self.seam(0x96bef5, self._throw)
        # Original static initializers of the global CStringW constants:
        # 9E73C0 B43B40 (empty), 9E73E0 B43B44 ("NULL"), 9E7400 B43B48 (empty),
        # 9E7420 B43B4C (" "); their atexit registration 96CAB0 is a seam.
        self.seam(0x96cab0, lambda: self.ret(0, 0))
        for init in (0x9e73c0, 0x9e73e0, 0x9e7400, 0x9e7420):
            self.call(init)

    def _throw(self):
        obj = self.arg(0)
        words = [self.r(obj + 4 * i) for i in range(8)]
        texts = []
        for w in words:
            if L.HEAP <= w < L.HEAP + L.HEAP_SIZE or 0x400000 <= w < 0xc00000:
                try:
                    t = self.text(w, 400)
                    if t and all(32 <= ord(c) < 0x3000 for c in t):
                        texts.append(t)
                except Exception:
                    pass
        caller = self.r(self.u.reg_read(L.UC_X86_REG_ESP))
        raise Thrown(f'C++ exception from {caller:#x}: {texts}')

    def _mbtowc(self):
        cp, flags, src, n, dst, cap = (self.arg(i) for i in range(6))
        assert cp == 0xfde9 and flags == 0, (cp, flags)
        data = bytes(self.u.mem_read(src, n))
        try:
            text = data.decode('utf-8')
        except UnicodeDecodeError:
            self.ret(0x18, 0)
            return
        raw = text.encode('utf-16le')
        if cap:
            self.u.mem_write(dst, raw[:cap * 2])
        self.ret(0x18, len(raw) // 2)

    def manager(self):
        # As the game data object does at 48AA9D: operator new(0x64) and the
        # KKC_TextDataManager ctor 4BAD50(0x1E), which stores itself in [B36544]
        # (1009 hash buckets, missing-string key "error_missing_string").
        mgr = self.alloc(0x64)
        self.call(0x4bad50, mgr, [0x1e])
        assert self.r(0xb36544) == mgr
        return mgr

    def load(self, chunk_bytes):
        buf = self.alloc(len(chunk_bytes) + 16)
        self.u.mem_write(buf, chunk_bytes)
        name = self.alloc(4)
        self.w(name, self.cstring('MAPSTRINGS'))
        reader = self.call(0x473050, 0, [name])
        self.w(reader + 0x30, buf)
        self.w(reader + 0x34, len(chunk_bytes))
        self.u.mem_write(reader + 0x44, b'\0')
        self.call(self.r(self.r(reader) + 0x4c), reader)
        root = self.r(reader + 0x3c)
        node = self.call(0x5b4410, root, [self.cstring('Text'), 0])
        if node == 0:
            return None, []
        mgr = self.manager()
        # LoadData registers into [B36544] (set by the ctor)
        textdata = self.alloc(0x24)
        cname = self.alloc(4)
        self.w(cname, self.cstring('map_strings'))
        self.call(0x475180, textdata, [cname])
        self.call(self.r(self.r(textdata)), textdata, [node])
        entries = []
        e = self.r(textdata + 0x18)
        while e:
            entry = self.r(e)
            entries.append([self.text(self.r(entry + 0xc)), self.text(self.r(entry + 0x10))])
            e = self.r(e + 4)
        return mgr, entries

    def lookup(self, mgr, key):
        slot = self.alloc(4)
        self.w(slot, self.cstring(key))
        found = self.call(0x4b9d40, mgr + 0x4c, [slot])
        if not found:
            return None
        empty = self.alloc(4)
        self.w(empty, self.cstring(''))
        text = self.call(0x4b9f40, mgr, [slot, empty])
        return self.text(self.r(text))


from mapstrings_cases import SYNTHETIC  # noqa: E402


def keys_for(chunk_name):
    """Localized trigger keys referenced by a map's TRIGGERS chunk."""
    p = HERE / 'chunks' / chunk_name
    if not chunk_name or not p.exists():
        return []
    out = []
    try:
        man = dc.Reader(p.read_bytes()).manager()
    except Exception:
        return []
    for t in man['triggers']:
        for label in ['events', 'conditions', 'actions']:
            for e in t[label]:
                for prm in e['parameters']:
                    if prm['storage'] == 7:
                        out.append(prm['value']['key'])
    return out


def run_case(name, raw, extra_keys):
    m = MapStrings()
    try:
        mgr, entries = m.load(raw)
    except Thrown as ex:
        return {'name': name, 'sha256': hashlib.sha256(raw).hexdigest(), 'error': str(ex)}, m
    lookups = {}
    if mgr is not None:
        probes = [k for k, _ in entries] + extra_keys + [k.upper() for k, _ in entries[:3]] + ['missing_key', 'ids', 'name']
        for k in probes:
            if k not in lookups:
                lookups[k] = m.lookup(mgr, k)
    return {'name': name, 'sha256': hashlib.sha256(raw).hexdigest(), 'text_node': mgr is not None,
            'entries': entries, 'lookups': lookups, 'diagnostics': m.diags, 'var_reads': m.var_reads}, m


def main():
    index = json.loads((HERE / 'chunks/index.json').read_text())
    by_map = {e['map']: e['file'] for e in index}
    rows, visited = [], {}
    last = None
    for p in sorted(DATA.rglob('*')):
        if p.suffix.lower() != '.rmp':
            continue
        rel = p.relative_to(DATA).as_posix()
        raw, pins = dc.chunk(p, 'MAPSTRINGS')
        row, m = run_case(rel, raw, keys_for(by_map.get(rel, '')))
        row['map_sha256'] = pins['map_sha256']
        rows.append(row)
        visited.update(m.visited)
        last = m
        print(rel, len(row.get('entries', [])), row.get('error', ''))
    for name, raw in SYNTHETIC.items():
        row, m = run_case('synthetic/' + name, raw, [])
        row['bytes_hex'] = raw.hex()
        rows.append(row)
        visited.update(m.visited)
        last = m
        print(name, row.get('entries', row.get('error')))
    last.visited = visited
    doc = {'schema': 1, 'original_sha256': L.pe.ORIGINAL_SHA, 'v14_sha256': L.pe.ACCEPTED_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'loader_sha256': hashlib.sha256((HERE / 'loader.py').read_bytes()).hexdigest(),
           'scope': __doc__.strip().splitlines()[0], 'cases': rows, 'visited_blocks': len(visited),
           'visited_windows': last.windows()}
    path = HERE / 'native-mapstrings.json'
    path.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
    print(json.dumps({'cases': len(rows), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
