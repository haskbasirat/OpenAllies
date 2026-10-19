"""Native 5A5650 KKC_NiSeqResource::ComputeLatestTimeController on every shipped KF.

Each .kf member of the original Data.rwd is loaded with the original
NiStream (native_nif_load); when LoadStream reaches 880E60 the first
top-level object gets one extra reference so it outlives the stream's
release. The original 5A5650 then runs on it; its 404ED0 diagnostics are a
seam that records the line number and returns. Output (--write):
native-kf-length.tsv (path, root class, length bits, reported lines).
"""
from pathlib import Path
import sys, os, json, struct, hashlib, time
from concurrent.futures import ProcessPoolExecutor

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
os.environ['NIF_CORPUS_EXT'] = '.kf'
import native_nif_load as N
import native_nif_corpus as C
import native_nif_fields as F
from unicorn import UC_HOOK_CODE

LENGTH = 0x5a5650
DIAGNOSTIC = 0x404ed0
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
    box = {}

    def snap(m):
        s = m.stream
        root = m.r(m.r(s + 0x148))
        m.w(root + 4, m.r(root + 4) + 1)
        box['root'] = root
        box['class'] = F.VT_NAMES.get(m.r(root))

    m = N.NifMachine()
    m.native_init()
    m.on_snapshot = snap
    stream = m.bump(0x200)
    m.call(N.NISTREAM_CTOR, ecx=stream)
    m.stream = stream
    buf = m.bump(len(data) + 16)
    m.u.mem_write(buf, data)
    ok = m.call(N.NISTREAM_LOAD_BUFFER, ecx=stream, args=(buf, len(data))) & 0xff
    m.stream = None
    assert ok == 1, path
    lines = []

    def diag(u, pc, sz, _):
        lines.append(m.arg(3))
        m.ret()

    m.u.hook_add(UC_HOOK_CODE, diag, begin=DIAGNOSTIC, end=DIAGNOSTIC)
    out = m.bump(16)
    stub = m.bump(32)
    code = b'\xff\x74\x24\x04' + b'\xe8' + struct.pack('<i', LENGTH - (stub + 4 + 5)) + b'\x83\xc4\x04' + b'\xd9\x1d' + struct.pack('<I', out) + b'\xc3'
    m.u.mem_write(stub, code)
    m.call(stub, args=(box['root'],))
    return path, box['class'], m.r(out), lines


def main():
    write = '--write' in sys.argv
    jobs = C.members()
    started = time.perf_counter()
    rows = []
    with ProcessPoolExecutor(max_workers=6) as pool:
        for path, cls, length, lines in pool.map(one, jobs, chunksize=4):
            rows.append((path, cls, '%08x' % length, ','.join('%x' % l for l in lines)))
    rows.sort(key=lambda r: r[0].lower())
    TAB, NL = chr(9), chr(10)
    tsv = ('path' + TAB + 'root' + TAB + 'length' + TAB + 'reports' + NL + ''.join(TAB.join(r) + NL for r in rows)).encode()
    if write:
        (HERE / 'native-kf-length.tsv').write_bytes(tsv)
    reports = {}
    for r in rows:
        if r[3]:
            reports[r[3]] = reports.get(r[3], 0) + 1
    print(json.dumps(dict(files=len(rows), reports=reports, seconds=round(time.perf_counter() - started, 1),
                          sha256=hashlib.sha256(tsv).hexdigest())))


if __name__ == '__main__':
    main()
