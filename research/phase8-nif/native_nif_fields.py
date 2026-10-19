"""Native object graph after NiStream::Load: field values and link lists.

Runs the native loader (native_nif_load.load) and, at the moment LoadStream
calls 880E60 (every object alive, linked, post-linked and post-processed),
reads the native objects' memory at the offsets their LoadBinary/LinkObject
functions use. One canonical record per streamed object:

  NiObjectNET     name (+0x0C C string), controller (+0x10)
  NiAVObject      flags (+0x20 u16), rotation (+0x38, 9 words),
                  translation (+0x5C, 3), scale (+0x68), property list
                  (+0xA4 head; node next +0, data +8)
  NiNode          children (NiTObjectArray +0xC4: data +0xC8, used +0xD0),
                  effect list (+0xE0 head)
  NiGeometry      data (+0xCC)
  NiGeometryData  vertex count (+0x0C u16), data flags (+0x30 u16),
                  consistency (+0x32 u16), bound (+0x10 centre, +0x1C radius),
                  SHA-256 of the vertex (+0x20, n*12), normal (+0x24, n*12 or
                  3n*12), colour (+0x28, n*16) and UV (+0x2C, sets*n*8) arrays
  NiTriBasedGeomData  triangle count (+0x3C u16)
  NiTimeController    flags (+0x0C u16), frequency/phase/start/stop
                  (+0x10..+0x1C), target (+0x34), next (+0x38)
  NiAlphaProperty flags (+0x20 u16), threshold (+0x22 u8)
  NiVertexColorProperty flags (+0x20 u16), vertex/lighting mode (+0x24, +0x28)
  NiZBufferProperty flags (+0x20 u16), function (+0x24)
  NiMaterialProperty  14 words at +0x24
  NiSourceTexture file name (+0x38), pixel data (+0x40), layout/mipmap/alpha
                  (+0x20..+0x28), static (+0x44 u8)
  NiTexturingProperty apply mode (+0x20), maps (+0x2C data, +0x34 size; map:
                  source +4, clamp +8, filter +0xC, UV set +0x10)
  NiTextKeyExtraData  keys (+0x10 count, +0x14 array; key: time +0, text +4)
  NiControllerSequence name (+0x0C), text-key name (+0xA8), text keys
                  (+0x10), target names (+0x18 data, +0x20 size), controllers
                  (+0x30 data, +0x38 size)
Object references are stream indices (None for NULL, 'x' for a pointer that
is not a streamed object). Files the old-particle post-process rewrites are
recorded by class only.

Output (--write): native-nif-fields.tsv (path, digest of the canonical JSON
records) and native-nif-fields-samples.json (full records of named samples);
with NIF_CORPUS_EXT=.kf, native-kf-fields.tsv / native-kf-fields-samples.json.
"""
from pathlib import Path
import sys, json, hashlib, struct, time
from concurrent.futures import ProcessPoolExecutor

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_nif_load as N
import native_nif_corpus as C

RTTI_KIND = None


def kind_table():
    """name -> base name, from the native RTTI static constructors."""
    md = N.capstone.Cs(N.capstone.CS_ARCH_X86, N.capstone.CS_MODE_32)
    rows = {}
    for f in N.rtti_initializers(N.ORIGINAL):
        seq = list(md.disasm(N.ORIGINAL.range(f, 32), f))[:4]
        base = int(seq[0].op_str, 16) if seq[0].op_str.startswith('0x') else 0
        name = N.ORIGINAL.range(int(seq[1].op_str, 16), 64).split(b'\0')[0].decode()
        rows[int(seq[2].op_str.split(',')[1], 16)] = (name, base)
    names = {k: v[0] for k, v in rows.items()}
    return {n: names.get(b) for n, b in rows.values()}


def is_kind(cls, base):
    while cls is not None:
        if cls == base:
            return True
        cls = RTTI_KIND.get(cls)
    return False


def sha(b):
    return hashlib.sha256(bytes(b)).hexdigest()


def dump(m, objects, classes):
    index = {p: i for i, p in enumerate(objects) if p}

    def ref(p):
        if p == 0:
            return None
        return index.get(p, 'x')

    def words(a, n):
        return list(struct.unpack('<%dI' % n, m.u.mem_read(a, 4 * n)))

    def u16(a):
        return m.r16(a)

    def u8(a):
        return m.u.mem_read(a, 1)[0]

    def cstr(p):
        return None if p == 0 else m.cstr(p, 4096).decode('latin-1')

    def plist(head):
        out, seen, node = [], set(), head
        while node:
            if node in seen:
                return 'cycle'
            seen.add(node)
            out.append(ref(m.r(node + 8)))
            node = m.r(node)
        return out

    def blob(p, n):
        return None if p == 0 else sha(m.u.mem_read(p, n))

    records = []
    for i, p in enumerate(objects):
        c = classes[i]
        r = [['class', c]]
        if p == 0 or c is None:
            records.append(r)
            continue
        if is_kind(c, 'NiObjectNET'):
            r.append(['name', cstr(m.r(p + 0xc))])
            r.append(['controller', ref(m.r(p + 0x10))])
        if is_kind(c, 'NiAVObject'):
            r.append(['flags', u16(p + 0x20)])
            r.append(['transform', words(p + 0x38, 9) + words(p + 0x5c, 3) + words(p + 0x68, 1)])
            r.append(['properties', plist(m.r(p + 0xa4))])
        if is_kind(c, 'NiNode'):
            data, used = m.r(p + 0xc8), m.r(p + 0xd0)
            r.append(['children', [ref(m.r(data + 4 * k)) for k in range(used)]])
            r.append(['effects', plist(m.r(p + 0xe0))])
        if is_kind(c, 'NiGeometry'):
            r.append(['data', ref(m.r(p + 0xcc))])
        if is_kind(c, 'NiGeometryData'):
            n = u16(p + 0xc)
            flags = u16(p + 0x30)
            normals = n * 3 if (flags >> 8) & 0xf0 else n
            r.append(['count', n])
            r.append(['data_flags', flags])
            r.append(['consistency', u16(p + 0x32)])
            r.append(['bound', words(p + 0x10, 4)])
            r.append(['arrays', [blob(m.r(p + 0x20), n * 12), blob(m.r(p + 0x24), normals * 12),
                                 blob(m.r(p + 0x28), n * 16), blob(m.r(p + 0x2c), (flags & 0x3f) * n * 8)]])
        if is_kind(c, 'NiTriBasedGeomData'):
            r.append(['triangles', u16(p + 0x3c)])
        if is_kind(c, 'NiTimeController'):
            r.append(['flags', u16(p + 0xc)])
            r.append(['times', words(p + 0x10, 4)])
            r.append(['target', ref(m.r(p + 0x34))])
            r.append(['next', ref(m.r(p + 0x38))])
        if c == 'NiAlphaProperty':
            r.append(['flags', u16(p + 0x20)])
            r.append(['threshold', u8(p + 0x22)])
        if c == 'NiVertexColorProperty':
            r.append(['flags', u16(p + 0x20)])
            r.append(['modes', words(p + 0x24, 2)])
        if c == 'NiZBufferProperty':
            r.append(['flags', u16(p + 0x20)])
            r.append(['function', m.r(p + 0x24)])
        if c == 'NiMaterialProperty':
            r.append(['material', words(p + 0x24, 14)])
        if c == 'NiSourceTexture':
            r.append(['file_name', cstr(m.r(p + 0x38))])
            r.append(['pixel_data', ref(m.r(p + 0x40))])
            r.append(['format', words(p + 0x20, 3)])
            r.append(['static', u8(p + 0x44)])
        if c == 'NiTexturingProperty':
            r.append(['apply', m.r(p + 0x20)])
            data, size = m.r(p + 0x2c), m.r(p + 0x34)
            maps = []
            for k in range(size):
                mp = m.r(data + 4 * k)
                maps.append(None if mp == 0 else [ref(m.r(mp + 4))] + words(mp + 8, 3))
            r.append(['maps', maps])
        if c == 'NiTextKeyExtraData':
            n, keys = m.r(p + 0x10), m.r(p + 0x14)
            r.append(['keys', [[m.r(keys + 8 * k), cstr(m.r(keys + 8 * k + 4))] for k in range(n)]])
        if c == 'NiControllerSequence':
            r.append(['name', cstr(m.r(p + 0xc))])
            r.append(['text_keys_name', cstr(m.r(p + 0xa8))])
            r.append(['text_keys', ref(m.r(p + 0x10))])
            data, size = m.r(p + 0x18), m.r(p + 0x20)
            r.append(['targets', [cstr(m.r(data + 4 * k)) for k in range(size)]])
            data, size = m.r(p + 0x30), m.r(p + 0x38)
            r.append(['sequence_controllers', [ref(m.r(data + 4 * k)) for k in range(size)]])
        records.append(r)
    return records


_PAIRS = None
VT_NAMES = None
TAB, NL = chr(9), chr(10)
POST_PROCESSED = {r['path']: bool(r['destroyed']) for r in json.loads((HERE / (C.PREFIX + '-corpus.json')).read_bytes())['rows']}


def vtable_names(pairs):
    """vtable -> NiRTTI class name of the object each create function builds."""
    m = N.NifMachine()
    m.native_init()
    out = {}
    for name, create in pairs:
        obj = m.call(create)
        out[m.r(obj)] = m.rtti_name(obj)
    return out


def one(job):
    global _PAIRS, RTTI_KIND, VT_NAMES
    path, off, size = job
    with open(N.GAME / 'Data.rwd', 'rb') as f:
        f.seek(off)
        data = f.read(size)
    if _PAIRS is None:
        _PAIRS = N.registry()
        RTTI_KIND = kind_table()
        VT_NAMES = vtable_names(_PAIRS)
    box = {}

    def snap(m):
        objects = m.snapshot
        classes = [None if p == 0 else VT_NAMES[vt] for p, vt in m.loaded]
        box['classes'] = classes
        # The old-particle post-process 8E93B0 has already replaced objects
        # here; the replaced ones die when 880E60 releases the stream's
        # references (native-nif-corpus.json 'destroyed'). Such files are
        # compared by class only.
        box['records'] = None if POST_PROCESSED.get(path) else dump(m, objects, classes)

    out = N.load(data, _PAIRS, vtables={}, on_snapshot=snap)
    assert out['ok'] == 1, path
    records = box['records'] if box['records'] is not None else [[['class', c]] for c in box['classes']]
    raw = json.dumps(records, separators=(',', ':')).encode()
    return path, sha(raw), (records if path in C.SAMPLES else None)


def main():
    write = '--write' in sys.argv
    jobs = C.members()
    refresh = [a.split('=', 1)[1] for a in sys.argv[1:] if a.startswith('--refresh=')]
    if refresh:
        # Recompute only the named files and merge them into the stored TSV.
        old = dict(l.split(TAB) for l in (HERE / (C.PREFIX + '-fields.tsv')).read_text().splitlines()[1:])
        for job in jobs:
            if job[0] in refresh:
                path, digest, _ = one(job)
                old[path] = digest
        tsv = ('path' + TAB + 'fields_sha256' + NL + ''.join(k + TAB + old[k] + NL for k in sorted(old, key=str.lower))).encode()
        (HERE / (C.PREFIX + '-fields.tsv')).write_bytes(tsv)
        print(json.dumps(dict(refreshed=refresh, sha256=sha(tsv))))
        return
    started = time.perf_counter()
    rows, samples = [], {}
    with ProcessPoolExecutor(max_workers=4) as pool:
        for i, (path, digest, records) in enumerate(pool.map(one, jobs, chunksize=4)):
            rows.append((path, digest))
            if records is not None:
                samples[path] = records
            if (i + 1) % 200 == 0:
                print(json.dumps(dict(done=i + 1, seconds=round(time.perf_counter() - started, 1))), flush=True)
    rows.sort(key=lambda r: r[0].lower())
    tsv = ('path\tfields_sha256\n' + ''.join('%s\t%s\n' % r for r in rows)).encode()
    sraw = (json.dumps(dict(schema=1, scope=__doc__, samples=samples), separators=(',', ':')) + '\n').encode()
    if write:
        (HERE / (C.PREFIX + '-fields.tsv')).write_bytes(tsv)
        (HERE / (C.PREFIX + '-fields-samples.json')).write_bytes(sraw)
    else:
        assert (HERE / (C.PREFIX + '-fields.tsv')).read_bytes() == tsv
    print(json.dumps(dict(files=len(rows), seconds=round(time.perf_counter() - started, 1), sha256=sha(tsv))))


if __name__ == '__main__':
    main()
