"""Oracle: ORIGINAL TRIGGERS chunk loading (TriggerManager 54D3D0) for every original map.

For each original map the MAPSTRINGS chunk is first loaded by the original
47EA40 path into a KKC_TextDataManager (native_mapstrings.MapStrings), then a
TriggerManager is built by its original constructor 54CEE0 and the TRIGGERS
chunk is read by 54D3D0 (manager versions 0..5) -> 541410 trigger -> 542A70 /
5468A0 element (5457E0 aliases, 5421C0 parameter array, 544190 upgrade rules)
-> 54A640 parameter -> 49AD70 localized pair (external mode: 4B9F40
GetDisplayString). The loaded native objects are read back field by field.

Registries: the original static constructors 546600 / 54A440 / 549D90 build the
element / argument / item registries at B3D474 / B3D498 / B3D4BC.
Seams (besides those of MapStrings): the stream (sub-object vtable[0] read =
copy from the chunk bytes, vtable+C IsStoring = 0, vtable+48 = 1) and 59E520 = 0 (the original RMP
load context: localized keys resolve through MAPSTRINGS), 48D500 (TGC data
registry lookup of the TriggerElementData named by the element; the returned
definition is filled by the original 4C6E40 from the registered index after
4C5DC0), 50D120 ("TriggerListUpdate" broadcast after loading; recorded).
A C++ throw (96BEF5) ends the case and is recorded as a native error.
Writes native-triggers.json.
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import loader as L  # noqa: E402
import native_mapstrings as nm  # noqa: E402

CHUNKS = HERE / 'chunks'


class TriggerLoad(nm.MapStrings):
    def __init__(self):
        super().__init__()
        for ctor, obj in [(0x546600, 0xb3d474), (0x54a440, 0xb3d498), (0x549d90, 0xb3d4bc)]:
            self.call(ctor, obj)
        self.registry = {}
        table = self.r(0xb3d478)
        for index in range(115):
            rec = self.r(table + 4 * index)
            assert self.r(rec) == index
            self.registry[self.text(self.r(rec + 4)).lower()] = index
        self.call(0x4c5dc0)
        self.definitions = {}
        for index in range(115):
            d = self.alloc(0x80)
            self.w(d + 12, index)
            self.call(0x4c6e40, d)
            assert self.r(d + 0x20) == index
            self.definitions[index] = d
        self.seam(0x48d500, self._definition)
        self.seam(0x59e520, lambda: self.ret(0, 0))
        self.broadcasts = []
        self.seam(0x50d120, lambda: (self.broadcasts.append(self.text(self.r(self.arg(0)))), self.ret(4)))
        ui = self.alloc(0x200)
        self.w(0xb36534, ui)
        # Stream: outer object S, sub-object at S+3C whose vtable[0] reads.
        self.stream = self.alloc(0x100)
        vt = self.alloc(0x100)
        self.w(self.stream + 0x3c, vt)
        self.w(vt, L.STUBS + 0x700)
        self.seam(L.STUBS + 0x700, self._read)
        self.w(vt + 0x48, L.STUBS + 0x710)
        self.seam(L.STUBS + 0x710, lambda: self.ret(0, 1))
        self.w(vt + 0xc, L.STUBS + 0x720)          # IsStoring: loading
        self.seam(L.STUBS + 0x720, lambda: self.ret(0, 0))
        self.data = b''
        self.at = 0

    def _read(self):
        dst, n = self.arg(0), self.arg(1)
        if self.at + n > len(self.data):
            raise nm.Thrown(f'stream read past end at {self.at} (+{n})')
        self.u.mem_write(dst, self.data[self.at:self.at + n])
        self.at += n
        self.ret(8, n)

    def _definition(self):
        name = self.text(self.arg(0))          # CStringW passed by value
        index = self.registry.get(name.lower()) if name else None
        self.ret(4, 0 if index is None else self.definitions[index])

    # ---- read back ----
    def cstr(self, p):
        return self.text(self.r(p))

    def nodes(self, head):
        out = []
        while head:
            out.append(self.r(head))
            head = self.r(head + 4)
        return out

    def parameter(self, q):
        storage = self.r(q)
        row = {'storage': storage, 'argument_index': self.r(q + 0x28), 'item_index': self.r(q + 0x2c),
               'flag': self.u.mem_read(q + 0x30, 1)[0]}
        if storage in (1, 2):
            row['word'] = self.r(q + 4)
        elif storage == 5:
            row['position'] = [self.r(q + 0x10 + 4 * i) for i in range(3)]
        elif storage == 6:
            row['text'] = self.cstr(q + 0x1c)
        elif storage == 7:
            row['key'] = self.cstr(q + 0x20)
            row['localized'] = self.cstr(q + 0x24)
        return row

    def element(self, e):
        d = self.r(e)
        count = self.r(e + 4)
        params = self.r(e + 8)
        return {'opcode': self.r(d + 0xc) if d else None, 'word_10': self.r(e + 0x10),
                'parameters': [self.parameter(params + 0x34 * i) for i in range(count)]}

    def trigger(self, t):
        return {'id': self.r(t + 8), 'mode': self.r(t), 'enabled': self.u.mem_read(t + 0x34, 1)[0],
                'repeat': self.u.mem_read(t + 0x35, 1)[0], 'activation_bits': self.r(t + 4),
                'name': self.cstr(t + 0xc), 'description': self.cstr(t + 0x10), 'word_2c': self.r(t + 0x2c),
                'events': [self.element(e) for e in self.nodes(self.r(t + 0x14))],
                'conditions': [self.element(e) for e in self.nodes(self.r(t + 0x1c))],
                'actions': [self.element(e) for e in self.nodes(self.r(t + 0x24))]}

    def variables(self, mgr):
        buckets, size = self.r(mgr + 0x30), self.r(mgr + 0x34)
        out = []
        for b in range(size):
            node = self.r(buckets + 4 * b)
            while node:
                var = self.r(node + 4)
                out.append({'name': self.cstr(var), 'bits': self.r(var + 4), 'key': self.cstr(node)})
                node = self.r(node + 8)
        assert len(out) == self.r(mgr + 0x38)
        return sorted(out, key=lambda v: (v['name'], v['bits']))

    def load_triggers(self, data):
        mgr = self.alloc(0x58)
        self.call(0x54cee0, mgr)
        self.data, self.at = data, 0
        self.call(0x54d3d0, mgr, [self.stream])
        strings = [self.cstr(self.r(mgr + 0x40) + 4 * i) for i in range(self.r(mgr + 0x44))]
        return {'consumed': self.at, 'next_id': self.r(mgr + 0xc),
                'triggers': [self.trigger(t) for t in self.nodes(self.r(mgr + 4))],
                'variables': self.variables(mgr), 'strings': strings, 'broadcasts': self.broadcasts}


def main():
    index = json.loads((CHUNKS / 'index.json').read_text())
    rows, visited, last = [], {}, None
    for entry in index:
        m = TriggerLoad()
        text = (CHUNKS / entry['mapstrings_file']).read_bytes()
        data = (CHUNKS / entry['file']).read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry['chunk_sha256']
        assert hashlib.sha256(text).hexdigest() == entry['mapstrings_sha256']
        row = {'map': entry['map'], 'chunk_sha256': entry['chunk_sha256'], 'mapstrings_sha256': entry['mapstrings_sha256']}
        try:
            mgr, _ = m.load(text)
            assert mgr is not None
            row.update(m.load_triggers(data))
        except nm.Thrown as ex:
            row['error'] = str(ex)
        row['diagnostics'] = m.diags
        rows.append(row)
        visited.update(m.visited)
        last = m
        print(entry['map'], len(row.get('triggers', [])), row.get('error', ''), m.diags[:3])
    last.visited = visited
    doc = {'schema': 1, 'original_sha256': L.pe.ORIGINAL_SHA, 'v14_sha256': L.pe.ACCEPTED_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'loader_sha256': hashlib.sha256((HERE / 'loader.py').read_bytes()).hexdigest(),
           'mapstrings_sha256': hashlib.sha256((HERE / 'native_mapstrings.py').read_bytes()).hexdigest(),
           'scope': __doc__.strip().splitlines()[0], 'cases': rows, 'visited_blocks': len(visited),
           'visited_windows': last.windows()}
    path = HERE / 'native-triggers.json'
    path.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
    print(json.dumps({'cases': len(rows), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
