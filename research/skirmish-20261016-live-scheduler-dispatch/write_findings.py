"""Record the frozen concrete scheduler service component after native/Rust replay."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    proof = json.loads((HERE/'native-dispatch.json').read_bytes())
    assert len(proof['cases']) == 972 and len(proof['noops']) == 135
    log = (HERE/'runtime-tests.log').read_text()
    assert '3 passed; 0 failed; 0 ignored' in log
    files = [ROOT/'crates/aa-runtime/src/live_scheduler_dispatch.rs',
             ROOT/'crates/aa-runtime/tests/live_scheduler_dispatch.rs']
    files += [HERE/name for name in ['native_dispatch.py','native-dispatch.json',
        'native-run.log','runtime-tests.log','runtime-tests.before-fixture-order-fix.log','README.txt']]
    result = {
        'schema':1,
        'status':'frozen concrete scheduled component service kernel; not complete world Services or battle readiness',
        'native_entries':{
            'periodic':'50B330 -> Updatable+14',
            'deferred3':'4F05C0 -> Updatable+18',
            'deferred4':'4F05B0 -> Updatable+1C (required/unbound)',
            'noop':'4D1E10 RET8 for Recruit/Builder/MobileDeployment/Research/HQ periodic vtables',
            'performer':'57F040 signed dispatch, actual4D03D0 pause sources,50ACC0 schedule and50B1C0 removal',
        },
        'authoritative_owners':['ActorStore component allocations','Owners portable Updatable identities',
            'Queues intrusive scheduling state','NativeRng stream','Spatial published placement'],
        'implemented_effects':[
            'actual no-op periodic leaves without consuming unused elapsed float words',
            'inactive/nonselected Performer returns before placement/Properties reads',
            'actual Thing+8 bit0 skips Properties; current Properties.modifier.flags at+1C bit1 controls pause',
            'paused delivery schedules same owner/data in actual queue3 with native epsilon and no RNG draw',
            'signed mismatched/default/negative events and cue with absent Character return natively',
            'stage0 stores actual turret_index=-1 and removes owner queue0/data0 before required57ED20 error',
        ],
        'remaining_required':[
            'Performer57ED20 begin','57E8C0 use frame','4C8300/4C8850 end ability context',
            '57DC90 actual Mind+138 notification','Animator563AA0 cue',
            'other periodic components','queue4 virtual+1C','Sun/Actor/Kingdom/Area/Doodad/Command/Controller updates',
            'other clocked::Services world effects/predicates before a real full tick',
        ],
        'native_proof':{
            'performer_rows':972,'noop_rows':135,'precisions':[24,53,64],
            'bounded_returns':756,'first_unbound_boundaries':216,
            'boundary_counts':{'begin':36,'use_frame':36,'end_context':36,'finish':36,'animator_cue':72},
            'actual_native_queue_helpers':True,'page0_unmapped':True,
            'seams':proof['seams'],
            'scope':'first unbound callee stopped at entry, never a fake successful callback',
        },
        'rust_validation':{
            'tests_passed':3,'failed':0,'ignored':0,
            'replayed_performer_rows':972,'replayed_noop_rows':135,
            'actual_original_source_actor':True,
            'isolated_noop_component_probes_qualified':True,
            'allocation_stays_in_place':True,
            'published_pose_not_snapshot':True,
            'actual_queued_paused_callback':True,
            'profile':'release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16',
            'initial_fixture_order_failure':'preserved; fixed constructor timing before publication without changing production guard',
        },
        'source_precision':'No fresh authored numeric fields read by admitted paths; future source-dependent updates require completed Bank source PC distinct from simulation PC',
        'independent_review':'not independently accepted at this freeze; native and author replay only',
        'limits':['not full component Update coverage','not movement/combat/economy gameplay success',
            'no client playability/multiplayer/save compatibility claim','not a full workspace or final EXE test',
            'errors preserve earlier state/queue effects; enclosing failed world requires recovery, not retry'],
        'all_owned_commands_reaped':True,
        'pins':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in files},
    }
    out = HERE/'findings.json'
    with out.open('x') as stream:
        json.dump(result,stream,indent=2)
        stream.write('\n')
    print({'findings_sha256':sha(out),'pins':len(files)})

if __name__ == '__main__':
    main()
