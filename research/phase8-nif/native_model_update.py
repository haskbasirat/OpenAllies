"""Native model-processing update: NiAVObject::Update(0.0, 1) on every shipped NIF.

87AA10 (KKC_NiProcessing::ProcessObject) calls NiAVObject::Update 86E8D0 at
87ABC0 with time 0.0 and controller updates enabled: NiNode
UpdateDownwardPass 866EB0 / NiAVObject 86EED0 -> UpdateObjectControllers
866BE0 (each Property's controllers via 86A520, then the object's own
controller chain, vtable +0x3C Update) -> UpdateWorldData 8A2C70 (45CAB0)
-> world bound; NiNode merges child bounds (89FAD0).

For each file and each x87 precision (PC24 0x007F, PC53 0x027F, PC64
0x037F) this loads the file with the original NiStream (native_nif_load) at
that precision, runs the original Update on the top-level object, and reads:
every NiAVObject's flags (+0x20), local transform (+0x38 rotation, +0x5C
translation, +0x68 scale), world transform (+0x6C, 13 words) and world bound
(+0x28 centre, +0x34 radius); every NiMaterialProperty's 14 colour words
(+0x24); every NiTimeController's clock words (+0x20 first, +0x24 last,
+0x28 accumulated, +0x2C scaled) and update byte +0x40; the last-key
indices of NiKeyframeController (+0x44 rotation, +0x48 position, +0x4C
scale) and NiAlphaController (+0x44); the Euler per-axis last-key indices
in key 0 (+0x40) of NiKeyframeData rotation type 4. The digest covers these
object records; the clock cache globals B44630/B44634/B44638/B4463C/B32454
(shared by every controller of the process) are a separate column.

Start-up static data the update reads is produced by the original static
initializers: 9E8380/9E83A0 (normalize constants B45E38/B45E3C, run at the
CRT default PC53). Any read of a .bss byte that nothing wrote is recorded
per file ("unwritten_bss"); a run that reads one is not a valid fixture.

Output (--write): native-model-update.tsv (path, pc, status, digest,
clock_cache),
native-model-update-samples.json (full records of the samples).
"""
from pathlib import Path
import sys, json, hashlib, struct, time
from concurrent.futures import ProcessPoolExecutor

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import native_nif_load as N
import native_nif_corpus as C
import native_nif_fields as F
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE

PCS = {'Single24': 0x007f, 'Double53': 0x027f, 'Extended64': 0x037f}
UPDATE = 0x86e8d0
STATIC_NUMERIC = [0x9e8380, 0x9e83a0]
BSS = (0xb37000, 0xb4c5c4)
CLOCK_GLOBALS = [0xb44630, 0xb44634, 0xb44638, 0xb4463c, 0xb32454]


def sha(b):
    return hashlib.sha256(bytes(b)).hexdigest()


class Taint:
    def __init__(self, m):
        self.written = set()
        self.unwritten = set()
        self.armed = False
        m.u.hook_add(UC_HOOK_MEM_WRITE, self._w, begin=BSS[0], end=BSS[1])
        m.u.hook_add(UC_HOOK_MEM_READ, self._r, begin=BSS[0], end=BSS[1])

    def _w(self, u, access, address, size, value, _):
        for a in range(address, address + size):
            self.written.add(a)

    def _r(self, u, access, address, size, value, _):
        if self.armed:
            for a in range(address, address + size):
                if a not in self.written:
                    self.unwritten.add(a)


def run(data, pc_name, pairs):
    m = N.NifMachine()
    taint = Taint(m)
    m.native_init()
    for f in STATIC_NUMERIC:
        m.call(f)
    taint.armed = True
    m.fpcw = PCS[pc_name]
    stream = m.bump(0x200)
    m.call(N.NISTREAM_CTOR, ecx=stream)
    m.stream = stream
    buf = m.bump(len(data) + 16)
    m.u.mem_write(buf, data)
    ok = m.call(N.NISTREAM_LOAD_BUFFER, ecx=stream, args=(buf, len(data))) & 0xff
    m.stream = None
    if ok != 1:
        return dict(status='load failed')
    objects = [p for p, vt in m.loaded]
    classes = [None if p == 0 else F.VT_NAMES[vt] for p, vt in m.loaded]
    top = m.r(m.r(stream + 0x148))
    try:
        m.call(UPDATE, ecx=top, args=(0, 1))
    except N.Abort as e:
        return dict(status='abort: %s' % e, unwritten_bss=sorted(taint.unwritten))
    index = {p: i for i, p in enumerate(objects) if p}

    def words(a, n):
        return list(struct.unpack('<%dI' % n, m.u.mem_read(a, 4 * n)))

    records = []
    for i, p in enumerate(objects):
        c = classes[i]
        r = [['class', c]]
        if p and F.is_kind(c, 'NiAVObject'):
            r.append(['flags', m.r16(p + 0x20)])
            r.append(['local', words(p + 0x38, 9) + words(p + 0x5c, 3) + words(p + 0x68, 1)])
            r.append(['world', words(p + 0x6c, 13)])
            r.append(['bound', words(p + 0x28, 4)])
        if p and c == 'NiMaterialProperty':
            r.append(['material', words(p + 0x24, 14)])
        if p and F.is_kind(c, 'NiTimeController'):
            r.append(['clock', words(p + 0x20, 4) + [m.u.mem_read(p + 0x40, 1)[0]]])
        if p and c == 'NiKeyframeController':
            r.append(['last_keys', words(p + 0x44, 3)])
        if p and c == 'NiAlphaController':
            r.append(['last_keys', words(p + 0x44, 1)])
        if p and c == 'NiKeyframeData' and m.r(p + 0x14) == 4 and m.r(p + 0x10):
            r.append(['euler_last_keys', words(m.r(p + 0x10) + 0x40, 3)])
        records.append(r)
    return dict(status='ok', records=records, clock_cache=[m.r(a) for a in CLOCK_GLOBALS],
                unwritten_bss=sorted(taint.unwritten))


_PAIRS = None


def one(job):
    global _PAIRS
    path, off, size = job
    with open(N.GAME / 'Data.rwd', 'rb') as f:
        f.seek(off)
        data = f.read(size)
    if _PAIRS is None:
        _PAIRS = N.registry()
        F.RTTI_KIND = F.kind_table()
        F.VT_NAMES = F.vtable_names(_PAIRS)
    rows = []
    for pc in PCS:
        if F.POST_PROCESSED.get(path):
            rows.append((path, pc, 'post-processed', '', '', None, []))
            continue
        out = run(data, pc, _PAIRS)
        digest = sha(json.dumps(out.get('records'), separators=(',', ':')).encode()) if out['status'] == 'ok' else ''
        cache = ' '.join('%08x' % w for w in out.get('clock_cache', []))
        sample = dict(records=out.get('records'), clock_cache=out.get('clock_cache')) if path in C.SAMPLES else None
        rows.append((path, pc, out['status'], digest, cache, sample, out.get('unwritten_bss', [])))
    return rows


def main():
    write = '--write' in sys.argv
    only = [a.split('=', 1)[1] for a in sys.argv[1:] if a.startswith('--only=')]
    jobs = [j for j in C.members() if not only or j[0] in only]
    started = time.perf_counter()
    rows, samples, unwritten = [], {}, {}
    with ProcessPoolExecutor(max_workers=6) as pool:
        for i, result in enumerate(pool.map(one, jobs, chunksize=2)):
            for path, pc, status, digest, cache, records, bss in result:
                rows.append((path, pc, status, digest, cache))
                if records is not None:
                    samples.setdefault(path, {})[pc] = records
                for a in bss:
                    unwritten.setdefault(a, []).append(path)
            if (i + 1) % 100 == 0:
                print(json.dumps(dict(done=i + 1, seconds=round(time.perf_counter() - started, 1))), flush=True)
    rows.sort(key=lambda r: (r[0].lower(), r[1]))
    TAB, NL = chr(9), chr(10)
    tsv = ('path' + TAB + 'pc' + TAB + 'status' + TAB + 'digest' + TAB + 'clock_cache' + NL +
           ''.join(TAB.join(r) + NL for r in rows)).encode()
    statuses = {}
    for r in rows:
        key = r[2] if not r[2].startswith('abort') else 'abort'
        statuses[key] = statuses.get(key, 0) + 1
    report = dict(files=len(jobs), statuses=statuses,
                  unwritten_bss={hex(a): sorted(set(v))[:5] for a, v in sorted(unwritten.items())},
                  seconds=round(time.perf_counter() - started, 1), sha256=sha(tsv))
    if write:
        (HERE / 'native-model-update.tsv').write_bytes(tsv)
        (HERE / 'native-model-update-samples.json').write_bytes(
            (json.dumps(dict(schema=1, scope=__doc__, samples=samples), separators=(',', ':')) + NL).encode())
    print(json.dumps(report, indent=1))


if __name__ == '__main__':
    main()
