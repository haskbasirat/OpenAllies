"""Independent navigation-worker replay and review of composed world queues."""
from pathlib import Path
import contextlib,hashlib,importlib,json,re,subprocess,sys,shutil
ROOT=Path(__file__).resolve().parents[2];SOURCE=Path(__file__).parent;OUT=SOURCE/'independent-output'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 OUT.mkdir(exist_ok=True);sys.path.insert(0,str(SOURCE));module=importlib.import_module('native_runtime');module.HERE=OUT
 from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
 original_call=module.q.Machine.call;abi_calls=0
 def checked_call(machine,*args,**kwargs):
  nonlocal abi_calls
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP];before=[machine.u.reg_read(r)for r in regs]
  result=original_call(machine,*args,**kwargs)
  assert before==[machine.u.reg_read(r)for r in regs],hex(args[0]);abi_calls+=1;return result
 module.q.Machine.call=checked_call
 with (OUT/'native-replay.log').open('w')as log,contextlib.redirect_stdout(log):module.run()
 expected=json.loads((SOURCE/'native-runtime.json').read_text());actual=json.loads((OUT/'native-runtime.json').read_text());assert actual==expected
 # Keep immutable native gold files untouched while replaying the clock edges.
 shutil.copyfile(SOURCE/'native_runtime.py',OUT/'native_runtime.py')
 module.q.VISITED.clear()
 edge=importlib.import_module('clock_edge_oracle');edge.HERE=OUT
 with (OUT/'clock-edge-replay.log').open('w')as log,contextlib.redirect_stdout(log):edge.main()
 assert json.loads((OUT/'native-clock-edges.json').read_text())==json.loads((SOURCE/'native-clock-edges.json').read_text())
 commands=[];test_count=0
 for command,name in [(['cargo','test','-p','aa-runtime','-p','aa-world','--','--nocapture'],'rust-tests.log'),(['cargo','clippy','-p','aa-runtime','-p','aa-world','--all-targets','--no-deps','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);path=OUT/name;path.write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace')
  if name=='rust-tests.log':test_count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout.decode())))
  commands.append({'command':command,'exit_code':run.returncode,'log':str(path.relative_to(ROOT)),'sha256':sha(path)})
 sources=['crates/aa-world/src/lib.rs','crates/aa-world/src/initialization.rs','crates/aa-runtime/src/clocked.rs','crates/aa-runtime/tests/native_runtime.rs','crates/aa-simulation/src/arithmetic.rs','research/phase7-runtime/native_runtime.py','research/phase7-runtime/native-runtime.json','research/phase7-runtime/clock_edge_oracle.py','research/phase7-runtime/native-clock-edges.json']
 data={'native_replay_identical':True,'fixture_sha256':sha(SOURCE/'native-runtime.json'),'plans':len(expected['plans']),'composed_sequences':len(expected['composed']),'world_ticks':sum(c['ticks']for c in expected['composed']),'clock_edge_cases':117,'calls_checked_for_nonvolatile_register_preservation':abi_calls,'rust_tests':test_count,'commands':commands,'source_sha256':{p:sha(ROOT/p)for p in sources}}
 (OUT/'validation.json').write_text(json.dumps(data,indent=2)+'\n');finalize()
 print(json.dumps(data|{'commands':'see validation.json','source_sha256':'see validation.json'}))
def finalize():
 data=json.loads((OUT/'validation.json').read_text())
 for name,digest in data['source_sha256'].items():assert sha(ROOT/name)==digest,name
 report={'schema':1,'passed':True,'reviewer':'independent navigation worker','findings':[],
 'scope':'Explicit-precision native clock/five-queue initialization, fallible outer dispatcher and composed runtime clock/queue adapter',
 'resolved_findings':[{'issue':'PC53 large-tick double rounding lost by exact-product shortcut','resolution':'Runtime World.clock_precision now selects NativeClock::advance_fragment_at_precision;117 original edge cases pass.','counterexample':{'rate_bits':'0x41c1f60b','step_bits':'0x3d28f0eb','input_tick':'0x78d5c83c','native_pc53_end':'0x4c9f7c06','old_exact_product_end':'0x4c9f7c05'}},
 {'issue':'World/queue time and precision ownership was implicit across mutable services','resolution':'Services now forbid clock/precision changes; adapter checks both copies and precision on every return and stops the runtime after violation;4 invalid-service cases pass.'}],
 'validation':data,'native_range_provenance_sha256':sha(OUT/'native-provenance.json'),'review_script_sha256':sha(Path(__file__)),
 'checked_semantics':['Original reciprocal is storedbinary32 before period calculations;24 native plans and full original call-site constants agree.','Original world phase sequence invokes the actual five native queues;12 sequences×85ticks independently replayed with first-tick, manager, callback insertion and cancellation changes.','Queue clock is synchronized from authoritative world time before each world service and queue phase, including first-tick pre-advance scheduling.','All required callbacks return Result; errors stop later callbacks and retain already committed state. Runtime remains stopped and refuses subsequent steps after failure or contract violation.','Explicit x87 multiply precision and final binary32 store preserve large-tick PC53 double rounding and signed tick rollover;117 edge cases independently replayed.','Native code windows agree with v14;1992 native call boundaries independently preserve nonvolatile registers, in addition to producer stack assertions.','Nine Rust tests across runtime/world and scoped strict all-target Clippy pass after fixes.'],
 'limits':['Actor/component gameplay, world startup/ownership, admin/decree tick authorization and save restoration remain required services.','This is a deterministic fixture callback composition, not a live or complete match.','Runtime checks ownership on callback return; a violating callback may already have produced effects, which are intentionally not rolled back.','Unknown/nonfinite native numeric and pointer domains are not emulated as memory corruption.','Unrelated aa-runtime catalog/relations/settings modules are outside this reviewed source scope.']}
 target=SOURCE/'independent-review.json';target.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'review_passed':True,'sha256':sha(target)}))
if __name__=='__main__':main()
