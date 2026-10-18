"""Original army territory changes on the InitCreate worlds.

For every shipped campaign a seeded sequence of army relocations is executed
with the original component method 940100 (956280 removal from the old
territory incl. the +1C update, 956140 slot insertion, 93F9F0 path append /
truncate). Half of the moves go back to a territory already in the army's
path. After each move the territory army lists, counts and the moved army's
territory and path are recorded. Relations: team based.
"""
import json, hashlib, random
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA, run, snapshot


def main():
    rng = random.Random(0x940100)
    rows = []
    for ci in range(len(DATA['campaigns'])):
        snap, ctx = run(ci, keep=True)
        m, w, st = ctx['m'], ctx['w'], ctx['st']
        territories = w.read_vector(st + 0x28)
        created = ctx['created']
        moves = []
        for step in range(60):
            a = rng.randrange(len(created))
            comp = m.r32(created[a]['actor'] + 0xcc)
            path = [territories.index(t) for t in w.read_vector(comp + 0x24)]
            if path and rng.random() < 0.5:
                t = rng.choice(path)
            else:
                t = rng.randrange(len(territories))
            m.call(0x940100, args=(territories[t],), ecx=comp)
            s = snapshot(ctx)
            moves.append({'army': a, 'territory': t,
                          'territories': [{'armies': [[r['army'], r['slot']] for r in tt['armies']],
                                           'enemy_army_count': tt['enemy_army_count']} for tt in s['territories']],
                          'army_territory': s['armies'][a]['territory'], 'army_path': s['armies'][a]['path']})
        rows.append({'campaign': ci, 'moves': moves})
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'sequences': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-moves.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'sequences': len(rows), 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
