"""Scan Rust crates and research oracles for native function addresses.

Read-only helper used by build_inventory.py. An address token is a 6-hex-digit
value 0x400000..0x9FFFFF that is a function start in functions.tsv and is not
embedded in a longer hexadecimal literal.
"""
from pathlib import Path
import collections, csv, re

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
NATIVE = GAME / 'reconstruction/research/native-aa'
RUN = re.compile(r'(?<![0-9A-Fa-f])[0-9A-Fa-f]+(?![0-9A-Fa-f])')


def function_starts():
    with open(NATIVE / 'functions.tsv', encoding='utf-8') as handle:
        return {int(r['address'], 16): r for r in csv.DictReader(handle, delimiter='\t')}


def scan(paths, starts):
    found = collections.defaultdict(set)
    for path in paths:
        text = path.read_text(encoding='utf-8', errors='ignore')
        for match in RUN.finditer(text):
            digits = match.group(0).lstrip('0')
            if len(digits) != 6 or digits[0] not in '456789':
                continue
            address = int(digits, 16)
            if address in starts:
                found[address].add(path.relative_to(ROOT).as_posix())
    return found


def crate_references(starts):
    return scan(sorted((ROOT / 'crates').rglob('*.rs')), starts)


def research_references(starts):
    paths = [p for p in (ROOT / 'research').rglob('*') if p.suffix in ('.py', '.txt', '.json')
             and p.stat().st_size < 4_000_000 and 'phase8-economy' not in p.as_posix()]
    paths += [p for p in (ROOT / 'research/phase8-economy').glob('native_*.py')]
    return scan(paths, starts)


if __name__ == '__main__':
    starts = function_starts()
    crates = crate_references(starts)
    print(len(crates), 'crate-referenced function starts')
    for a in sorted(crates):
        print(f'{a:06x}', ' '.join(sorted(crates[a])[:3]))
