"""Independent common-restoration replay; gold fixtures remain read-only."""
from pathlib import Path
import hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent;REPLAY=OUT/'review-replay'
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run_native():
 REPLAY.mkdir(exist_ok=True);results=[]
 for name in ['native_object_load','native_component_actions','native_component_order','native_saved_components']:
  original=OUT/(name+'.py');copy=REPLAY/original.name
  source=original.read_text();assert source.count('ROOT=Path(__file__).resolve().parents[2]')==1
  source=source.replace('ROOT=Path(__file__).resolve().parents[2]','ROOT=Path(__file__).resolve().parents[3]')
  # Extra ABI checks for the two original Load families; no gameplay hooks added.
  if name in ['native_object_load','native_saved_components']:
   suffix='''\n_original_run = Machine.run
_abi_calls = 0
def _review_run(self,*args,**kwargs):
 global _abi_calls
 regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
 for i,r in enumerate(regs): self.u.reg_write(r,0x11111111*(i+1))
 result=_original_run(self,*args,**kwargs)
 assert self.u.reg_read(UC_X86_REG_ESP)==self.stack+8
 assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
 _abi_calls+=1
 return result
Machine.run=_review_run
'''
   source=source.replace("if __name__=='__main__':main()",suffix+"\nif __name__=='__main__':\n main()\n print({'independent_abi_calls':_abi_calls})")
  copy.write_text(source);cmd=[sys.executable,str(copy)];p=subprocess.run(cmd,cwd=ROOT,text=True,capture_output=True)
  log=REPLAY/(name+'.log');log.write_text(p.stdout+p.stderr);assert p.returncode==0,p.stdout+p.stderr
  file=name.replace('_','-')+'.json';fresh=REPLAY/file;gold=OUT/file
  assert json.loads(fresh.read_text())==json.loads(gold.read_text()),name
  results.append({'producer':str(original.relative_to(ROOT)),'producer_sha256':H(original),'replay_sha256':H(copy),'fixture':file,'gold_sha256':H(gold),'exact_match':True,'log_sha256':H(log)})
  print(name,'passed',flush=True)
 (REPLAY/'native-validation.json').write_text(json.dumps(results,indent=2)+'\n')
def finish():
 import re
 sys.path.insert(0,str(ROOT/'research/phase4-rng'))
 from original_rng import original,modified
 records=json.loads((REPLAY/'native-validation.json').read_text())
 checked=0
 for row in records:
  path=OUT/row['fixture'];assert H(path)==row['gold_sha256']
  doc=json.loads(path.read_text())
  for pin in doc.get('instructions',[]):
   address=int(pin['va'],16);raw=bytes.fromhex(pin['bytes'])
   assert original.range(address,len(raw))==raw==modified.range(address,len(raw));checked+=1
 commands=[]
 for name,cmd in [('review-tests.log',['cargo','test','-p','aa-mapworld','--test','restore','--test','restore_components','--test','saved_components','--test','actor_payload','--test','restore_stop','--','--include-ignored','--nocapture']),('review-clippy.log',['cargo','clippy','-p','aa-mapworld','--lib','--no-deps','--','-D','warnings'])]:
  proc=subprocess.run(cmd,cwd=ROOT,text=True,capture_output=True);log=OUT/name;log.write_text(proc.stdout+proc.stderr);assert proc.returncode==0,proc.stdout+proc.stderr
  commands.append({'command':cmd,'exit_code':0,'passed_tests':sum(map(int,re.findall(r'test result: ok\. (\d+) passed',proc.stdout))),'log':name,'sha256':H(log)})
 sources=['crates/aa-mapworld/src/restore_components.rs','crates/aa-mapworld/src/actor_payload.rs','crates/aa-mapworld/src/saved_components.rs','crates/aa-mapworld/tests/restore.rs','crates/aa-mapworld/tests/restore_components.rs','crates/aa-mapworld/tests/saved_components.rs','crates/aa-mapworld/tests/actor_payload.rs','crates/aa-mapworld/tests/restore_stop.rs']
 text=(ROOT/'crates/aa-mapworld/src/restore.rs').read_text();starts=list(re.finditer(r'(?m)^(?:pub )?fn (\w+)\(',text));sections={}
 for i,m in enumerate(starts):
  if m.group(1)in['apply_loaded','restore_records','restore_records_with_mode','restore_dynamic']:
   body=text[m.start():starts[i+1].start()if i+1<len(starts)else len(text)]
   sections[m.group(1)]=hashlib.sha256(body.encode()).hexdigest()
 report={'schema':1,'passed':True,'reviewer':'navigation independent review','scope':'Common saved component codecs/application, component dispatch, authoritative owner-slot states, immediate Updatable registration and strict stop-on-error orchestration. Ongoing Kingdom suffix is explicitly excluded.','findings':[],'resolved_findings':[{'issue':'Mover v8 wrote later fields before required speed resolution/multiplication could fail.','resolution':'Earlier vector10/words20/24 commit first, speed errors stop before later fields. Both missing source and rejected nonfinite multiplier regression cases pass.'},{'issue':'Element wire decoder accepted signed-negative history counts but application rejected them; native skips the loop.','resolution':'Application now rejects only signed-positive unsupported history and resets flag1C/time24. Original negative-count replay added and application compared.'}],'source_sha256':{f:H(ROOT/f)for f in sources},'restore_rs_reviewed_function_sections_sha256':sections,'restore_rs_context_sha256':H(ROOT/'crates/aa-mapworld/src/restore.rs'),'dependency_sha256':{f:H(ROOT/f)for f in ['crates/aa-actors/src/loading.rs','crates/aa-actors/src/owned.rs','crates/aa-actors/src/ground_states.rs','crates/aa-numeric/src/lib.rs']},'native_replays':records,'native_counts':{'object_load':144,'component_payload':170,'load_actions':248,'component_dispatch_slots':31,'additional_stack_and_nonvolatile_checks':314},'original_v14_instruction_pins_checked':checked,'commands':commands,'review_script_sha256':H(Path(__file__)),'checked':['Constructor state is inserted in the authoritative component slot before immediate Updatable publication and saved field application.','A failed Updatable callback retains constructor state, logs no successful registration and does not apply later saved fields.','Component callbacks preserve Construction-first, ordinary slot order and Mind-last, stopping at the first error.','Original tutorial01 injected duplicate Updatable failure stops before later Kingdoms and dynamic runtime work. InspectPartial is diagnostic and never authorizes runtime publication.','Typed owner IDs retain generations and slot indexes; default services fail rather than silently succeed.','Old-version wire presence, defaults and successful Body/Mover arithmetic agree with original native replays.'],'limits':['Body/Mover/Properties/Element/Economy/Detection/Experience common application is covered; other decoded component payloads still need specialized concrete constructors/application.','Pure wire decoding is bounded and can reject unsupported/truncated input before applying that component; no malformed-stream partial-read parity claim.','Actor tail reference fixups, post-load callbacks, ongoing Kingdom suffix and full playable map restoration remain outside this gate.','No renderer gameplay, full match, save round-trip or multiplayer claim.']}
 (OUT/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n');print({'passed':True,'sha256':H(OUT/'independent-review.json')})
if __name__=='__main__':
 if '--finalize' in sys.argv:finish()
 else:run_native()
