from pathlib import Path
import hashlib,json,shutil
folder=Path(__file__).resolve().parent;root=folder.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
files=['crates/aa-runtime/src/native_camera_ni.rs','crates/aa-runtime/src/native_camera_ni_tests.rs','research/skirmish-20261017-native-camera/native_camera_ni.py','research/skirmish-20261017-native-camera/native-camera-ni.json']
snapshot=folder/'ni-source-snapshot';snapshot.mkdir(exist_ok=True)
for name in files:shutil.copyfile(root/name,snapshot/Path(name).name)
shutil.copyfile(root/'target/s-borrowed-leader/native-camera-ni-tests.exe',folder/'frozen-ni-tests.exe')
fixture=json.loads((folder/'native-camera-ni.json').read_text())
report={
 'result':'PASS bounded same-owner NiCamera field math',
 'production_and_oracle_pins':{name:sha(root/name)for name in files},
 'production_check':{'result':'pass','seconds':31.21,'log_sha256':sha(folder/'ni-production-check-01.log')},
 'native_direct':{'paired_rows':150,'separate_original_v14_executions':300,'precision':['Single24','Double53','Extended64'],'block_windows':64,'immutable_replay_log_sha256':sha(folder/'ni-native-replay.log'),'services':'Parent45CAB0 and attached+B0 virtual38 stop at required service entries. No successful substitute children. Heap/logger transports from Loader exist but are not called by the null-parent/null-attachment arithmetic.','inputs':'Actual KKC_Camera vtableA84850; arbitrary finite local matrix/translation plus opaque raw scale payload, finite nondegenerate frustum; explicit parent24/B0 guards. This is a laboratory prefill, not a base constructor.'},
 'rust':{'binary_sha256':sha(folder/'frozen-ni-tests.exe'),'owned_tests_passed':3,'combined_tests_passed':6,'combined_seconds':0.12,'combined_log_sha256':sha(folder/'ni-combined-replay.log'),'additional_existing_native_pose_rows':180,'existing_pose_fixture_sha256':sha(folder/'native-camera-pose.json'),'scope':'Exact source is compiled with rustc against accepted numeric/trig/serde libraries, no whole-runtime unit link. Full Ni output slice is compared for existing180 native rows using only child-unmodified local/frustum inputs. New150 direct paired rows test arbitrary perspective/orthographic branches and required-service prefixes.'},
 'same_owner':['8A2C70 copies actual local13 to world13 when actual parent24 is null','positive parent transform requires real child before world stores','positive attached vf38 occurs after world stores and before basis/planes/matrix','8742D0 bases+114..134,874240sixplanes,8737B0projection/viewmatrix,873790position,6FE050fourcorners mutate same State','unwritten108 stays unbound; no Camera164 root writes'],
 'native_leaves':['8A2C70','8742D0','874240','873300','873380','873430','873500','8735E0','8736B0','872F00','873000','873100','873200','42D750','869C90','8737B0','873790','6FE050'],
 'numeric':['Pinned9FD38C threshold358637BD,1 at9F32E0 and-1 at9FDC48','Native binary32 temp boundaries and extended x87 intermediates retained; sqrt/inverse stay extended','Z,Y,X dot reduction; asymmetric perspective vector and corner temporary stores','Ortho depth expression intentionally follows actual native inverse-of-stored-inverse, not conventional substitute matrix'],
 'limits':['This does not construct7016A0/NiCamera or authorize initialized VWorld/renderer/battle','Non-null parent45CAB0 and attached+B0 owners remain required; positive error-prefix paths are proved, not positive backend success','Arithmetic proof is finite, nondegenerate-frustum domain; exceptional nonfinite/overflow/invalid native pointer and x87 status-register behavior are not certified','Per-instruction stop injection into every private numeric operation is not claimed; exact service-boundary prefixes and full bounded arithmetic outputs are proved','Original180-row pose fixture includes declared raw80 hardware trig transport; the new direct Ni fixture needs no trig transport'],
 'history':'Initial native producer expected Loader.windows bytes, but that helper returns digests only; fixed to retain actual visited block lengths/bytes. Failed author log preserved. Fixture then generated and fresh default immutable replay passed without mutation.',
}
(folder/'ni-findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'report_sha256':sha(folder/'ni-findings.json'),'source_pins':report['production_and_oracle_pins'],'binary':report['rust']['binary_sha256']},indent=2))
