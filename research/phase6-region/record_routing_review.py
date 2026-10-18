"""Record independent inspection, native replay, and focused Rust validation."""
from pathlib import Path
import hashlib,json

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
replay=json.loads((HERE/'review-routing-replay.json').read_text(encoding='utf-8'))
assert replay['passed'] and replay['cases']==516
assert replay['reviewer_sha256']==sha(HERE/'review_routing.py')
logs={name:(HERE/name).read_text(encoding='utf-8-sig') for name in ['review-routing-tests.log','review-routing-clippy.log']}
assert logs['review-routing-tests.log'].count('test result: ok.')==4
assert '2 passed; 0 failed' in logs['review-routing-tests.log']
assert 'Finished' in logs['review-routing-clippy.log'] and 'error:' not in logs['review-routing-clippy.log']
paths=[HERE/'review_routing.py',HERE/'record_routing_review.py',HERE/'review-routing-replay.json',
       HERE/'review-routing-replay.log',HERE/'review-routing-tests.log',HERE/'review-routing-clippy.log']
ledgers=['research/phase6-region/region-evidence.json','research/phase6-region/graph-evidence.json',
         'research/phase6-pathsearch/pathsearch-findings.json','research/phase6-pathcost/pathcost-findings.json']
for path in ledgers:
    p=ROOT/path;doc=json.loads(p.read_text(encoding='utf-8'));paths.append(p)
    for key in ['source_sha256','dependency_sha256','pins']:
        for rel,digest in doc.get(key,{}).items():
            source=ROOT/rel
            assert sha(source)==digest,rel
            paths.append(source)
paths.extend([ROOT/'research/phase4-rng/original_rng.py',ROOT/'research/phase6-navigation/materialize_oracle.py',
              ROOT/'research/phase3-terrain/terrain_uv_oracle.py'])
for module in ['region.rs','region_search.rs','search.rs','cost.rs']:
    paths.append(ROOT/'crates/aa-navigation/src'/module)
for test in ['region_prepass.rs','region_search_native.rs','search_native.rs','cost_native.rs']:
    paths.append(ROOT/'crates/aa-navigation/tests'/test)
report={
    'schema':1,'passed':True,'reviewer':'world_diagnostic (independent of routing authors)',
    'scope':['region prepass 53DC00','uncached concrete region search 53F320/53EDE0',
             'concrete tile search 53D8D0','wall-entry and terrain-change cost 53CB70'],
    'native_cases':516,'counts':{'region_prepass':168,'region_graph':96,'tile_search':75,'tile_cost':177},
    'rust_tests':5,'strict_clippy_passed':True,
    'inspection':[
        'Compared source with native instructions for prepass pruning, asymmetric normalization spills, head-coordinate/payload update and nontransactional endpoint failure.',
        'Verified concrete region vtable binding, fixed start/goal heuristic, distance-cache/fallback spill, first-discovery close, category gate, byte-epoch rollover and goal-based finalizer.',
        'Verified heap equality order, parent-facing neighbor fan, both diagonal orthogonal queries, goal detection before secondary collision/cost, depth cutoff and extended closest comparison.',
        'Verified wall-entry identity gates, fresh owner after virtual callback, captured source tile address, stored wall weighting and terrain penalty operation/store ordering.',
        'Checked error propagation and explicit invalid predecessor behavior; no implicit rollback or fake successful fallback.',
        'region_route provides the typed region chain; cost supplies the full callback arithmetic. The independent replay validates components and contracts, not an actual-map end-to-end world route.'
    ],
    'native_validation':'All immutable authored outputs replayed in reverse order. Every visited original instruction byte matched accepted v14; added callee-preserved register sentinels for tile search and cost. Native region producers already check ABI.',
    'rust_validation_command':'cargo test -p aa-navigation --test region_prepass --test region_search_native --test search_native --test cost_native -- --nocapture',
    'clippy_command':'cargo clippy -p aa-navigation --all-targets -- -D warnings',
    'findings':[],
    'limits':[
        'Actual-map region topology construction, world-to-region lookup and cached-route flag1 tables remain absent.',
        'Prepass world/collision/endpoint services and tile-search dynamic collision are required callbacks. Native tile-search replay stops at final waypoint materialization.',
        'Costs require real generation-resolved actors, Body health, terrain definitions and SVars. The search corpus exercises the no-wall/no-terrain branch; full cost branches are independently tested.',
        'Finite numeric domain, valid graph/grid storage and explicit x87 precision only; no full FPU exception lifecycle or native undefined memory behavior is asserted.',
        'Optional unbounded queues alter native capacity policy; using the documented native limits preserves that policy. Native depth limits remain.',
        'Existing Solve/controller/materializer implementation is a pinned dependency, not a newly reviewed scope. Shared module exports are pinned only as dependencies.',
        'Native ABI checks belong to original execution, not Rust ABI compatibility.'
    ],
    'sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sorted(set(paths))}
}
for directory,component in [('phase6-region','region prepass and uncached graph'),('phase6-pathsearch','concrete tile search'),('phase6-pathcost','wall-entry and terrain-change cost')]:
    output=ROOT/'research'/directory/'independent-review.json'
    output.write_text(json.dumps({**report,'this_report_component':component},indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'file':output.relative_to(ROOT).as_posix(),'sha256':sha(output)}))
