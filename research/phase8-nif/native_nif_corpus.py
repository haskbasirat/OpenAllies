"""Run the native NiStream load oracle (native_nif_load.py) over every shipped NIF.

Input: every `.nif` member of the pinned original Data.rwd, read through the
member table printed by the rwd tool; each member is checked byte-identical to
the extracted loose copy (reconstruction/assets/original/Data). The archive is
never written.

Output (--write): native-nif-corpus.json with, per file, the native
LoadStream result, error code/text, header version/user version, the native
class of every streamed object (RTTI name, digest + counts), the top-level
object indices, and SHA-256 digests of the exact read trace (offset, requested,
delivered, load index, link-ID list length, link-count list length), the
link-ID list, the link-count list and the LinkObject pop trace. Plus the
executed original instruction ranges, verified identical in the accepted v14
image. Full traces of a few named samples go to native-nif-samples.json.
Without --write the stored file is regenerated and compared.
"""
from pathlib import Path
import sys, json, hashlib, struct, subprocess, collections, time
from concurrent.futures import ProcessPoolExecutor

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
sys.path.insert(0, str(HERE))
import native_nif_load as N

SAMPLES = ['Special/Things/ThingStartLocation/ThingStartLocation.nif', 'Special/Empty.nif',
           'Effects/Gunshot/RifleShotExplosion1.NIF', 'Bridges/ArcticBridge/ArcticBridge01.nif',
           'Buildings/Campaign/BeachMinePole/BeachMinePole.nif', 'Doodads/Temperate/TemperateLakeMist/TemperateLakeMist01.nif']


def members():
    exe = None
    for c in sorted((ROOT / 'target').glob('*/release/rwd.exe')) + sorted((ROOT / 'target').glob('*/*/release/rwd.exe')):
        exe = c
        break
    if exe is None:
        raise SystemExit('build the aa-formats rwd tool first')
    out = subprocess.run([str(exe), 'list', str(GAME / 'Data.rwd')], capture_output=True, check=True).stdout.decode('utf-8')
    rows = []
    for line in out.splitlines()[1:]:
        src, size, off, path = line.split('\t')
        if path.lower().endswith('.nif'):
            rows.append((path, int(off), int(size)))
    return rows


def sha(b):
    return hashlib.sha256(b).hexdigest()


_PAIRS = None


def one(job):
    global _PAIRS
    path, off, size = job
    with open(GAME / 'Data.rwd', 'rb') as f:
        f.seek(off)
        data = f.read(size)
    loose = (GAME / 'reconstruction/assets/original/Data' / path).read_bytes()
    assert loose == data, path
    if _PAIRS is None:
        _PAIRS = N.registry()
    row = dict(path=path, size=size, sha256=sha(data))
    try:
        out = N.load(data, _PAIRS, vtables={})
    except Exception as e:
        row['abort'] = '%s: %s' % (type(e).__name__, e)
        return row, None
    classes = out['classes']
    row.update(ok=out['ok'], error_code=out['error_code'], error=out['error'], version=out['version'],
               user_version=out['user_version'], objects=len(classes),
               classes_sha256=sha('\n'.join(c or '' for c in classes).encode()),
               class_counts=dict(sorted(collections.Counter(classes).items())), roots=out['roots'],
               destroyed=out['destroyed'],
               reads=len(out['reads']), consumed=sum(r[2] for r in out['reads']),
               short_reads=sum(1 for r in out['reads'] if r[2] != r[1]),
               trace_sha256=N.trace_digest(out['reads']),
               link_ids=len(out['link_ids'] or []), link_ids_sha256=N.words_digest(out['link_ids'] or []),
               link_counts=len(out['link_counts'] or []), link_counts_sha256=N.words_digest(out['link_counts'] or []),
               link_consumed=list(out['link_consumed'] or ()),
               pops=len(out['pops']),
               pops_sha256=sha(json.dumps(out['pops'], separators=(',', ':')).encode()))
    sample = None
    if path in SAMPLES:
        sample = dict(path=path, sha256=row['sha256'], classes=classes, roots=out['roots'], reads=out['reads'],
                      link_ids=out['link_ids'], link_counts=out['link_counts'], pops=out['pops'])
    return row, (out['blocks'], sample)


def main():
    write = '--write' in sys.argv
    workers = 6
    for a in sys.argv[1:]:
        if a.startswith('--workers='):
            workers = int(a.split('=')[1])
    jobs = members()
    started = time.perf_counter()
    rows, blocks, samples = [], {}, []
    with ProcessPoolExecutor(max_workers=workers) as pool:
        for i, (row, extra) in enumerate(pool.map(one, jobs, chunksize=4)):
            rows.append(row)
            if extra:
                for a, s in extra[0].items():
                    if blocks.get(a, 0) < s:
                        blocks[a] = s
                if extra[1]:
                    samples.append(extra[1])
            if (i + 1) % 100 == 0 or 'abort' in row:
                print(json.dumps(dict(done=i + 1, total=len(jobs), seconds=round(time.perf_counter() - started, 1),
                                      last=row['path'], abort=row.get('abort'))), flush=True)
    # Pin every executed original instruction block; the v14 image must be identical there.
    pins = []
    for a, s in sorted(blocks.items()):
        b0 = N.ORIGINAL.range(a, s)
        b1 = N.V14.range(a, s)
        assert b0 is not None and b0 == b1, hex(a)
        pins.append([a, s])
    pin_bytes = b''.join(N.ORIGINAL.range(a, s) for a, s in pins)
    versions = collections.Counter((r.get('version'), r.get('user_version')) for r in rows)
    doc = dict(schema=1, scope=__doc__, original_sha256=sha(N.ORIGINAL.b), v14_sha256=sha(N.V14.b),
               producer_sha256=sha(Path(__file__).read_bytes()), oracle_sha256=sha((HERE / 'native_nif_load.py').read_bytes()),
               files=len(rows), aborted=sum('abort' in r for r in rows), loaded=sum(r.get('ok') == 1 for r in rows),
               version_counts={'%08x/%s' % (v or 0, u): c for (v, u), c in sorted(versions.items(), key=lambda x: (x[0][0] or 0))},
               executed_blocks=len(pins), executed_bytes=len(pin_bytes), executed_sha256=sha(pin_bytes),
               rows=sorted(rows, key=lambda r: r['path'].lower()))
    out = HERE / 'native-nif-corpus.json'
    sample_out = HERE / 'native-nif-samples.json'
    raw = (json.dumps(doc, indent=1) + '\n').encode()
    sraw = (json.dumps(dict(schema=1, samples=sorted(samples, key=lambda s: s['path'])), separators=(',', ':')) + '\n').encode()
    if write:
        out.write_bytes(raw)
        sample_out.write_bytes(sraw)
    else:
        assert out.read_bytes() == raw, 'stored corpus oracle differs'
        assert sample_out.read_bytes() == sraw, 'stored samples differ'
    print(json.dumps(dict(files=doc['files'], loaded=doc['loaded'], aborted=doc['aborted'], versions=doc['version_counts'],
                          blocks=len(pins), seconds=round(time.perf_counter() - started, 1), sha256=sha(raw))))


if __name__ == '__main__':
    main()
