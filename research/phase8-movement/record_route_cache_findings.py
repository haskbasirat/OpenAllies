"""Write native-route-cache-findings.json from native-route-cache.json.

Static facts below were read from the original instructions (capstone over
AA.exe, cross-checked against v14 by harness.py when executed); the case
statistics and branch coverage are computed from the oracle fixture.
"""
import hashlib, json
from pathlib import Path
HERE = Path(__file__).resolve().parent
FIXTURE = HERE / 'native-route-cache.json'

BRANCHES = {
    '0x53E610 cached lookup': 0x53e618, '0x53E610 min/max swap': 0x53e630,
    '0x53F230 -> 0x53F120 record': 0x53f244, '0x53EBD0 flag-1 finish': 0x53ebda,
    '0x53EBD0 flag-0 finish': 0x53ebf9,
    '0x53EAC0 direct': 0x53ead0, '0x53EAC0 walk loop': 0x53eaf0,
    '0x53EAC0 landing (0x7B90C0)': 0x53eb19, '0x53EAC0 overshoot': 0x53eb33,
    '0x53F120 cur<to': 0x53f164, '0x53F120 cur>=to': 0x53f186,
    '0x53F120 end from<to': 0x53f1c7, '0x53F120 end from>=to': 0x53f1ec,
    '0x53E920 start==goal': 0x53e96d, '0x53E920 0xFE no route': 0x53e963,
    '0x53E920 insert after (0x53E750)': 0x53e9a0, '0x53E920 inline push_front': 0x53e9be,
    '0x53E920 insert before (0x5C2C70)': 0x53ea22, '0x53E920 push_back (0x53E6D0)': 0x53ea36,
    '0x53E920 end insert after': 0x53ea7e, '0x53E920 end push_front (0x53E680)': 0x53eaa0,
    '0x53E5B0 empty list': 0x53e5fd, '0x53E5B0 accumulate': 0x53e5d0,
    '0x5D8560 free rows': 0x5d856e, '0x7ABC30 destroy old cache': 0x7abca5,
    '0x7AAC90 rows destructor': 0x7aac98, '0x53ED50 eh vector ctor 0x96D18D': 0x96d18d,
    '0x96BE33 eh vector dtor': 0x96be33, '0x4F0590 fail cleanup': 0x4f0590,
}


def main():
    raw = FIXTURE.read_bytes()
    doc = json.loads(raw)
    visited = {int(i['va'], 16) for i in doc['instructions']}
    coverage = {k: (a in visited) for k, a in BRANCHES.items()}
    assert all(coverage.values()), coverage
    stats = {'searches_flag1': 0, 'searches_flag0': 0, 'cache_hits_start_ne_goal': 0,
             'cache_misses_full_search': 0, 'start_eq_goal': 0, 'status1_empty_no_route': 0,
             'status0': 0, 'native_faults': 0, 'path_costs': 0, 'empty_list_costs': 0,
             'nonfinite_costs': 0, 'creates': 0, 'resets': 0, 'destroys': 0}
    for c in doc['cases']:
        for o, r in zip(c['ops'], c['results']):
            kind = o['op']
            if kind in ('create', 'reset', 'destroy'):
                stats[kind + 's'] += 1
            if kind != 'search':
                continue
            stats['searches_flag%d' % o['flag']] += 1
            if 'fault' in r:
                stats['native_faults'] += 1
                continue
            stats['path_costs'] += 1
            stats['empty_list_costs'] += r['route'] == []
            stats['nonfinite_costs'] += (r['cost_bits'] >> 23) & 0xff == 0xff
            if o['flag'] != 1:
                continue
            if r['status'] == 0:
                stats['status0'] += 1
            elif o['start'] == o['goal']:
                stats['start_eq_goal'] += 1
            elif not r['popped']:
                stats['cache_hits_start_ne_goal'] += 1
                stats['status1_empty_no_route'] += r['route'] == []
            else:
                stats['cache_misses_full_search'] += 1
    findings = {
        'schema': 1,
        'scope': 'SAI route cache [[0xB3652C]+0x40], region search 0x53F320 flag 1, path cost 0x53E5B0 '
                 '(inputs of SAI actor travel time 0x7A1B30/0x7A1C10)',
        'ported': {
            '0x53F320 flag 1': 'entry stores output list/start/goal/flag into solver+0x44/+0x38/+0x3C/+0x40, i.e. the '
                               'embedded strategy (solver+0x34) fields +0x10/+0x04/+0x08/+0x0C -> '
                               'route_cache::search_cached (shared 0x53EDE0 engine region_search::run)',
            '0x53E610': 'strategy+0x2C early test: flag 1 -> start==goal, else cache byte (min,max) != 0xFF '
                        '(reads [mgr+0x40] only when start != goal)',
            '0x53F230': 'strategy+0x14 mark: flag 1 and from != to -> 0x53F120 first, then 0x7B90C0/0x7B8E30 '
                        'epoch+predecessor bytes (as flag 0)',
            '0x53F120': 'cache record: back = 0x7B90C0(to, from); walk cur from start to from by 0x53EAC0 hops; '
                        'cur<to: byte(cur,to) = 0x53EAC0(cur,from) else byte(to,cur) = back; '
                        'end: from<to: byte(from,to) = 0x7B90C0(from,to) else byte(to,from) = back',
            '0x53EAC0': 'hop byte at x towards y: x<y direct byte; else walk y towards x via (y,x) bytes until '
                        'y>=x; overshoot -> byte(x,y); landing on x -> 0x7B90C0(x, last y); x==y reads region[-1]',
            '0x53EBD0 flag 1': 'strategy+0x10 finish -> 0x53E920(cache, start, goal, list, 0); result discarded',
            '0x53E920': 'clear list (0x5C2CD0); start==goal -> [start]; byte(start,goal)==0xFE -> false, empty; '
                        'else two-ended walk, the smaller end advances by its direct byte; start side inserted '
                        'after the front cursor (inline push_front / 0x53E750), goal side before the back cursor '
                        '(0x53E6D0 push_back / 0x5C2C70), meeting region last',
            '0x7B90C0': 'adjacency slot of a region pointer, 0xFE when absent',
            '0x53E5B0': 'fastcall list {head,tail}: head null -> binary32 [0x9FDC48] = -1.0 (0xBF800000); else '
                        'local=+0.0; for each node until tail: ECX = region[cur id], arg = next id, '
                        'FADD m32 local to the 0x7BB420 result at PC, FSTP m32; return FLD m32 local -> '
                        'route_cache::path_cost (explicit Precision)',
            '0x53F0A0/0x53ED50': 'cache build from manager count [+0x18] (signed, <=1 builds nothing): count=n-1, '
                                 'row i = {n-1-i, bytes}, filled 0xFF -> RouteCache::build / reset',
            '0x7ABC30 (cache tail)': 'destroy old object (0x7AAC90 body inline + free), new 8-byte {0,0} '
                                     '(0x79E130), [mgr+0x40] = it, 0x53F0A0 -> RouteCache::build',
            '0x5D8560': 'ResetPathPlanner console command (registered at 0x5D8C92, name 0xA4F1A4): free rows if '
                        'count>0, rebuild with 0x53F0A0 from the current count; no null check -> '
                        'RouteCache::reset / route_cache::reset_path_planner',
            '0x7AC9A0 tail 0x7ACAC8..0x7ACAE5': 'destroy: 0x7AAC90 + free + [mgr+0x40]=0 -> caller drops the cache',
            '0x7A1BA3..0x7A1BEC': 'fresh list, 0x53F320(list,start,goal,1) on solver [[0xB36534]+0x234], '
                                  '0x53E5B0, FSTP m32, 0x5C2CD0 -> route_cache::route_cost (composition only)',
        },
        'cache_lifecycle': {
            'object': '[[0xB3652C]+0x40] = {count, rows}; rows = vector-new array (count prefix at rows-4) of '
                      '{size, bytes}; byte for pair (i<j) at [rows+i*8+4] - i + j - 1 = next-hop adjacency slot '
                      'of i towards j; 0xFF unknown',
            'constructor': '0x7AA0B0 manager ctor sets +0x40 = 0 (0x7AA168)',
            'create': '0x7ABC30 (only caller 0x7A0D40 at 0x7A0D7F, ECX=[0xB3652C]); 0x7A0D40 is the SAI ready '
                      'object [0xB36528] method that returns early when ready, and sets ready +0x20 = 1 at '
                      '0x7A0E25 afterwards; called from 0x7757F0 (0x775B63, 0x775BC3)',
            'reset': '0x5D8560 ResetPathPlanner console command only',
            'destroy': '0x7AC9A0 manager clear (0x7ACAC8..0x7ACAE2), called from destructor 0x7ACE50 (0x7ACE7C) '
                       'and from topology build/load 0x7ACB00 (0x7ACBF7, when manager +4 is set); when the manager '
                       'was finalised (+5, set by 0x7ABC30) it first calls 0x79FF10 which clears ready +0x20 '
                       '(0x79FFC3)',
            'writers': 'only 0x53F120 (flag-1 mark) writes bytes; readers 0x53E610, 0x53E920, 0x53EAC0',
            'all [mgr+0x40] accesses': ['0x7AA168 ctor', '0x7ABC9E..0x7ABD1A create', '0x7ACAC8..0x7ACAE2 destroy',
                                        '0x53E61D early', '0x53EBEC finish', '0x53F24E mark',
                                        '0x5D8566/0x5D85A7 reset'],
            'invalidation': 'none besides reset/recreate/destroy: entries survive epoch wraps, flag-0 searches and '
                            'any later search; a cached answer is the path in the search tree that wrote it '
                            '(fixture grid_stale_trees: 6->2 after 2->6 is [6,3,0,1,2], a fresh search gives '
                            '[6,7,8,5,2])',
        },
        'flag1_callers': ['0x5728D4', '0x5D88BF', '0x7A1BD1 (0x7A1B30 travel time)', '0x7A274C', '0x7AA872',
                          '0x7B8045', '0x7C9F8F'],
        'flag0_callers': ['0x53DC96 (region prepass 0x53DC00)'],
        'busy_byte': '0xB3D43D is process-wide: read/set by 0x4F4050 (water flood fill, '
                     'aa_spatial::water_areas::FloodSolver::busy), 0x539650, 0x53BE30, 0x53D8D0, 0x53EDE0 (both '
                     'modes; region_search::State::busy), 0x5402E0, 0x960FB0. Entering any of them while it is set '
                     'fails (status 0) and clears it. One shared flag reproduces the native interlock.',
        'native_undefined_to_errors': {
            'null cache, start != goal (fault 0x53E636 / 0x53F24E path)': 'CacheError::Missing',
            'hop byte >= adjacency count, incl. 0xFF unknown and 0xFE (fault 0x53EA0E)': 'CacheError::InvalidHop',
            'pair outside rows (graph grew after build)': 'CacheError::PairOutOfRange',
            'non-terminating cached walk (native loops, allocating list nodes)': 'CacheError::Cycle',
            '0x53EAC0 with x == y (region[-1])': 'CacheError::EqualHopPair (unreachable from native callers)',
            '0x53F120 diagonal write (cur == to)': 'CacheError::Diagonal (unreachable: to is undiscovered)',
            '0x53F120 reading a pair it rewrites': 'CacheError::SelfOverlap (unreachable: all its reads are '
                                                   'ancestor/descendant pairs of the current search tree)',
            'status 2 in flag 1 (queue limit / depth 255 with best != start)':
                'finish reads the still-unknown (start,goal) byte -> InvalidHop',
            '0x53E5B0 +inf/NaN result': 'aa_numeric NonFiniteInput or Binary32Overflow',
        },
        'caps': {
            'removed': 'none in this port',
            'retained': {'queue limit 0x2000': '0x53F290 (0x53F2F8) for solver [[0xB36534]+0x234]; '
                                               'Request::queue_limit Some(NATIVE_QUEUE_LIMIT) reproduces it, '
                                               'None removes it (unchanged region_search convention)',
                         'u8 hop/predecessor bytes and u8 adjacency count': 'native data widths of the cache and '
                                                                            'region nodes (RegionGraph::validate)'},
            'note': 'the cache has no size cap: count-1 rows from the signed 32-bit region count',
        },
        'seams': doc['seams'],
        'precisions': ['Single24', 'Double53', 'Extended64'],
        'snan_rows': 'cost_snan_* (snan_input true): sNaN cached distance on the final hop; native 0x7FE00000, '
                     'Rust NonFiniteInput',
        'parallelism': 'flag-1 searches mutate the cache, epoch and busy byte: run them in native order',
        'unproven_or_unported': [
            '0x7A1B30/0x7A1C10 actor parts (vtable +0x8/+0xC/+0x10/+0x20, cached +0x14) belong to the AI owner',
            'per-region finalisers 0x7BCA60/0x7B9900/0x7BC8B0 inside 0x7ABC30 were seamed, not executed',
            '0x7AC9A0 other teardown and 0x79FF10 readiness clear: static evidence only',
            'which game events call 0x7A0D40/0x7ACB00 at runtime (map load order) is not traced here',
            'allocation order/addresses of rows and list nodes are not modelled (Vec ownership)',
            'Cycle/Diagonal/SelfOverlap/EqualHopPair guards are not reachable from native-built caches; only '
            'Cycle and PairOutOfRange have synthetic Rust tests',
        ],
        'fixture': FIXTURE.name,
        'fixture_sha256': hashlib.sha256(raw).hexdigest(),
        'producer_sha256': doc['producer_sha256'],
        'case_count': doc['case_count'],
        'case_stats': stats,
        'native_faults': doc['native_faults'],
        'branch_coverage': coverage,
        'executed_instructions': len(doc['instructions']),
        'rust': {
            'module': 'crates/aa-navigation/src/route_cache.rs (+ shared engine in region_search.rs)',
            'tests': 'crates/aa-navigation/tests/route_cache_native.rs',
            'commands': ['CARGO_TARGET_DIR=target/s-movement cargo test -p aa-navigation --release --locked '
                         '--offline -j 4',
                         'CARGO_TARGET_DIR=target/s-movement cargo clippy -p aa-navigation --all-targets '
                         '--no-deps --release --locked --offline -j 4 -- -D warnings'],
            'logs': ['route-cache-rust-tests.log', 'route-cache-clippy.log'],
            'results': 'cargo test -p aa-navigation: 40 passed, 0 failed, 4 ignored (map-dependent); '
                       'route_cache_native 3/3, region_search_native 2/2 unchanged. clippy -D warnings: clean '
                       'for --all-targets before another owner added tests/uai_collision_native.rs; afterwards '
                       'clean for --lib + the route_cache/region_search tests, and the only --all-targets error is '
                       'clippy::drop_non_drop in that other file (see the clippy log)',
        },
    }
    out = HERE / 'native-route-cache-findings.json'
    out.write_text(json.dumps(findings, indent=1) + '\n')
    print(json.dumps({'file': out.name, 'stats': stats}))


if __name__ == '__main__':
    main()
