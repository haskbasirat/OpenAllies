"""Original owner changes and victory tests on the InitCreate worlds.

For every shipped campaign (state from 939B90 via mg_campaign) a seeded
sequence of owner changes is applied with the original set-owner 955620
(959F20 leave old owner, 95A850 append, 954CF0 recount); after each change
the original victory test 95A6B0 and capital-lost test 95A350 are evaluated
for every kingdom, and the kingdoms' owned-territory lists and the territory
counts are recorded. Relations: team based (enemy = other team) or, for half
of the sequences, a random enemy matrix (diagonal false).
"""
import json, hashlib, random
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
from mg_campaign import DATA, run, snapshot


def main():
    rng = random.Random(0x95a6b0)
    rows = []
    for ci in range(len(DATA['campaigns'])):
        for variant in range(4):
            snap, ctx = run(ci, keep=True)
            m, w, st = ctx['m'], ctx['w'], ctx['st']
            n = len(ctx['actors'])
            if variant % 2:
                rel = [[a != b and rng.random() < 0.6 for b in range(n)] for a in range(n)]
                w.relations['enemy_140'] = rel
            enemy = w.relations['enemy_140']
            kingdoms = w.read_vector(st + 0x1c)
            territories = w.read_vector(st + 0x28)
            caps = [territories.index(m.r32(k + 0x10)) for k in kingdoms]
            steps = []
            for step in range(12):
                # favour capitals so that victories actually happen
                t = rng.choice(caps) if rng.random() < 0.6 else rng.randrange(len(territories))
                k = rng.randrange(n)
                m.call(0x955620, args=(kingdoms[k],), ecx=territories[t])
                wins = [m.call(0x95a6b0, ecx=kp)[0] & 0xff for kp in kingdoms]
                lost = [m.call(0x95a350, ecx=kp)[0] & 0xff for kp in kingdoms]
                s = snapshot(ctx)
                steps.append({
                    'territory': t, 'new_owner': k, 'wins': wins, 'capital_lost': lost,
                    'kingdom_territories': [kk['territories'] for kk in s['kingdoms']],
                    'owners': [tt['owner'] for tt in s['territories']],
                    'enemy_counts': [tt['enemy_army_count'] for tt in s['territories']],
                })
            rows.append({'campaign': ci, 'variant': variant, 'enemy_140': enemy, 'steps': steps})
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'sequences': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-victory.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    wins = sum(sum(s['wins']) > 0 for r in rows for s in r['steps'])
    print(json.dumps({'sequences': len(rows), 'steps_with_a_winner': wins, 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
