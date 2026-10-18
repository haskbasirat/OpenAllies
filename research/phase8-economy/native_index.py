"""Read-only index over the exported native inventories (functions/calls/strings)."""
from pathlib import Path
import collections, csv, sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
NATIVE = GAME / 'reconstruction/research/native-aa'


def _rows(name):
    with open(NATIVE / name, encoding='utf-8') as handle:
        yield from csv.DictReader(handle, delimiter='\t')


FUNCTIONS = {int(r['address'], 16): r for r in _rows('functions.tsv')}
CALLERS = collections.defaultdict(set)
CALLEES = collections.defaultdict(set)
for r in _rows('calls.tsv'):
    if r['callee'].startswith('EXTERNAL'):
        continue
    try:
        caller, callee = int(r['caller'], 16), int(r['callee'], 16)
    except ValueError:
        continue
    CALLERS[callee].add(caller)
    CALLEES[caller].add(callee)
STRINGS = collections.defaultdict(list)
UNOWNED_STRINGS = []
for r in _rows('string-references.tsv'):
    if r['functionAddress']:
        STRINGS[int(r['functionAddress'], 16)].append(r['value'])
    else:
        UNOWNED_STRINGS.append((int(r['fromAddress'], 16), r['value']))


def owner(address):
    """Function containing an instruction address, by start/size from functions.tsv."""
    best = None
    for start, row in FUNCTIONS.items():
        if start <= address < start + int(row['bodyBytes']):
            if best is None or start > best:
                best = start
    return best


def describe(address):
    row = FUNCTIONS.get(address)
    size = int(row['bodyBytes']) if row else None
    return {
        'address': f'{address:08x}', 'size': size,
        'strings': sorted(set(STRINGS.get(address, [])))[:8],
        'callees': [f'{c:08x}' for c in sorted(CALLEES.get(address, ()))],
        'callers': [f'{c:08x}' for c in sorted(CALLERS.get(address, ()))],
    }


if __name__ == '__main__':
    for arg in sys.argv[1:]:
        d = describe(int(arg, 16))
        print(d['address'], d['size'], '|'.join(d['strings']))
        print('   ->', ' '.join(d['callees']))
        print('   <-', ' '.join(d['callers']))
