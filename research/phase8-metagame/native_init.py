"""Original KKC_MGState::InitCreate (939B90) on every shipped campaign.

Native objects are built from metagame-data.json (exported by the Rust loader
test `export_definitions_for_oracles`) at their native offsets:
  campaign: +08 ids, +18 map, +1C map table, +44 kingdoms, +50 events
  campaign kingdom (0x68): +04 ids, +10 capital map territory, +24 starting
      resources, +54 territory records {+0 liberates_to, +4 map territory,
      +8 actor data vector}
  map: +28 territories (0x7C: +04 ids, +0C type, +10 production)
  MGState as constructed by 939690 (vectors, 25-bucket hash, RNG 0x8E4B12A5).
The trig lookup tables (9C8620/9C8640) and the actor_spacing_type enum
table (959B20/959A50) are built by the original static initializers.
Seams (engine side, recorded): world 4F48E0 kingdom lookup by ids (returns a
kingdom actor whose +14 handle and +18 name string = campaign kingdom IDS are
synthetic engine inputs), 4F9C10 territory visual creation, 4FA030 army actor creation
(the seam builds an army actor with a zeroed MetaGame component), 939230 map
table props, CRT heap. Kingdom relations: enemy = different campaign team,
self is never an enemy, 138-relation = same team.
Everything else (MG kingdom/territory constructors, hash insert, owner change
955620, army registration 940100/956140/93F9F0, 955B70 placement incl. the
TerritoryPositionSolver) is original code.
"""
import json, hashlib
from pathlib import Path
from mg_machine import P, V, VISITED, HERE
import mg_campaign
from mg_campaign import DATA, run

def main():
    rows = [run(i) for i in range(len(DATA['campaigns']))]
    report = {
        'schema': 1, 'scope': __doc__,
        'original_sha256': hashlib.sha256(P.b).hexdigest(), 'v14_sha256': hashlib.sha256(V.b).hexdigest(),
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'data_sha256': hashlib.sha256((HERE / 'metagame-data.json').read_bytes()).hexdigest(),
        'campaigns': rows,
        'instructions': [{'va': hex(a), 'bytes': b} for a, b in sorted(VISITED.items())],
    }
    out = HERE / 'native-init.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'campaigns': len(rows), 'territories': len(rows[0]['territories']),
                      'armies': [len(r['armies']) for r in rows], 'visited': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))

if __name__ == '__main__':
    main()
