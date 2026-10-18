"""Write research/phase8-zones/findings.json: fixture counts and hashes,
producer and Rust source pins, test/clippy results, native quirks, removed
caps and open gaps. Read-only inputs (run the producers first)."""
import hashlib, json, re
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

FIXTURES = {
    'native-zone-sets.json': ('native_zone_sets.py', 'crates/aa-zones/tests/zone_sets_native.rs'),
    'native-zone-queries.json': ('native_zone_queries.py', 'crates/aa-zones/tests/zone_queries_native.rs'),
    'native-zone-tiles.json': ('native_zone_tiles.py', 'crates/aa-zones/tests/zone_tiles_native.rs'),
    'native-zone-moves.json': ('native_zone_moves.py', 'crates/aa-zones/tests/zone_moves_native.rs'),
    'native-zone-flood.json': ('native_zone_flood.py', 'crates/aa-zones/tests/zone_flood_native.rs'),
    'native-detection.json': ('native_detection.py', 'crates/aa-zones/tests/detection_native.rs'),
    'native-detection-component.json': ('native_detection_component.py', 'crates/aa-zones/tests/detection_component_native.rs'),
    'native-zone-enter.json': ('native_zone_enter.py', 'crates/aa-zones/tests/zone_enter_native.rs'),
    'native-kingdom-masks.json': ('native_kingdom_masks.py', 'crates/aa-zones/tests/kingdom_masks_native.rs'),
}


def main():
    fx = []
    for name, (producer, test) in FIXTURES.items():
        p = HERE / name
        d = json.loads(p.read_text())
        fx.append({'fixture': name, 'sha256': sha(p), 'producer': producer, 'producer_sha256': sha(HERE / producer),
                   'producer_sha256_recorded': d.get('producer_sha256'), 'rust_test': test,
                   'cases': d.get('case_count'),
                   'operations': d.get('op_count', d.get('query_count')), 'native_faults': d.get('faults'),
                   'executed_instructions': len(d.get('instructions', [])), 'coverage': d.get('coverage')})
    src = {str(p.relative_to(ROOT)).replace('\\', '/'): sha(p)
           for p in sorted((ROOT / 'crates/aa-zones').rglob('*.rs'))}
    src['crates/aa-zones/Cargo.toml'] = sha(ROOT / 'crates/aa-zones/Cargo.toml')
    log = (HERE / 'rust-tests.log').read_text(errors='replace') if (HERE / 'rust-tests.log').exists() else ''
    passed = sum(int(m) for m in re.findall(r'test result: ok\. (\d+) passed', log))
    failed = sum(int(m) for m in re.findall(r'(\d+) failed', log))
    inv = json.loads((HERE / 'inventory.json').read_text())
    out = {
        'schema': 1,
        'date': '2026-10-15',
        'scope': 'Original AA.exe zones (Zones.cpp), ZoneSolver, tile fog/supply words, AI zone queries, actor detection (GObject)',
        'original_sha256': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
        'v14_sha256': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
        'fixtures': fx,
        'rust_sources': src,
        'tests': {'command': 'CARGO_TARGET_DIR=target/s-zones cargo test -p aa-zones --release --locked --offline -j 4',
                  'passed': passed, 'failed': failed,
                  'clippy': 'cargo clippy -p aa-zones --release --locked --offline -j 4 --tests --no-deps -- -D warnings: clean'},
        'inventory': {'functions': inv['function_count'], 'status_counts': inv['status_counts']},
        'precision': 'x87-dependent paths (fast builder, flood costs, clamp, actor hide) proven at PC24, PC53 and PC64; the simulation runs at PC53',
        'removed_caps': [
            {'address': '0x53CDF0', 'what': 'ZoneSolver open-list limit', 'native_value': '0x1F9C (8092 entries)',
             'rust': 'aa_zones::flood::SolverState::queue_limit (None in production, Some(0x1F9C) in the replay)'}],
        'kept_rules': [
            {'address': '0xA11740', 'what': '16 kingdom bits per fog/supply/reveal word', 'why': 'data layout of the words and of saved games'},
            {'address': '0x4D69A0', 'what': '9 zone types per actor', 'why': 'gameplay rule (zone type set)'}],
        'svars': {'ZoneBuilderFastDetectionZones': 'true in Data/Game/SVars.tgi (fast circle builder for types 1, 2, 6); kept as a world input'},
        'native_quirks': [
            'Zone-set interning (0x5161C0 + 0x5154E0) merges [a,b] and [b,a]: the iteration order of a tile set depends on which equal set is alive anywhere (ported exactly, mutation-tested).',
            '0x516560 union remove of an absent actor walks into the final leaf and corrupts memory (Error::NativeAbsentActor); 0x516460 leaf remove ignores the actor and yields (actor, flags ^ mask).',
            'Fast span callbacks (0x53FD30/0x53FDE0/0x53FF00) clip the end column to width-1 exclusively: the last map column never receives a fast-built zone.',
            '0x540020: old r^2 from the register radius, new r^2 from the stored binary32; old raster radius inv*r_reg vs new r32*inv: boundary tiles of a later remove pass can differ from the add pass.',
            'Flood release 0x4F4050 removes from the start tile even when the zone never covered it (centre on an impassable tile): null dereference or XOR/UB (189 oracle cases).',
            'ZoneSolver reads its start tile +0x24 without reset; the FIFO solver stores tile pointers there; a link read as a cost is address dependent (assumed tile array below 0x3F800000; not exercised).',
            '0x5153B0 union slot +0x3C returns list-empty while the leaf returns added.',
            '0x54F6A0 counts the SAI occupied counter down even when the supply bit was already clear.',
            'ZoneSolver neighbour steps have no bounds check (relies on an impassable border); outside the tile array the port errors.',
            'CRT _CIfmod with a tile size below 1.0 (q = 0) enters the CRT error path; the port rejects it (division by zero).',
            'KKC_Zone::Move clears every zone of a dead actor except types 1/6 (logged by 0x404F60).',
        ],
        'open_gaps': [
            'Detection component reveal sources 0x56EDC0 (engagement lists) and the modifier context of 0x45D790 (0x501C30) are seams.',
            'Zone creators in components; GObject init/load/copy (0x4EBE50, 0x4EC7A0, 0x4EC360), cheat fog 0x4F9970, reveal-all 0x551F90/0x551FD0.',
            'Runtime binding: aa-runtime does not call aa-zones; storage for zones, actor +0x54 words and memory slots, tile +0x20/+0x1C data and the shared solver scratch.',
        ],
    }
    (HERE / 'findings.json').write_text(json.dumps(out, indent=1))
    print('findings.json', passed, 'passed', failed, 'failed')


if __name__ == '__main__':
    main()
