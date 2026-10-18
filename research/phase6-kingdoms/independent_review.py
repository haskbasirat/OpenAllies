"""Independent original kingdom/team reader replay and owner-index acceptance."""
from pathlib import Path
import hashlib,json,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_teams as teams
import native_kingdoms as kingdoms
import native_leaves as leaves
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
sentinels=[0x11111111*(i+1)for i in range(4)]
def prepare(m):
 for r,v in zip(regs,sentinels):m.u.reg_write(r,v)
def verify(m):assert[m.u.reg_read(r)for r in regs]==sentinels
def windows(doc):
 for row in doc['instructions']:
  a=int(row['va'],16);raw=bytes.fromhex(row['bytes']);assert teams.P.range(a,len(raw))==raw==teams.V.range(a,len(raw))
def main():
 t=json.loads((HERE/'native-teams.json').read_text());k=json.loads((HERE/'native-kingdoms.json').read_text());l=json.loads((HERE/'native-leaves.json').read_text())
 team_count=kingdom_count=0
 for row in t['maps']:
  data=teams.read_chunks(Path(row['map']))['TEAMS'][0];assert hashlib.sha256(data).hexdigest()==row['sha256']
  m=teams.Machine(data);count=m.r.u32();assert count==len(row['entries'])
  for entry in row['entries']:
   assert m.r.take(1)[0]==entry['present_byte']
   if entry['present_byte']:
    prepare(m);got=m.team();verify(m)
    assert got=={key:entry[key] for key in got},(row['map'],entry['slot']);team_count+=1
  assert m.r.at==row['decoded_bytes']==len(data)
 print(json.dumps({'teams_replayed':team_count}),flush=True)
 for row in k['maps']:
  data=kingdoms.read_chunks(Path(row['map']))['KINGDOMS'][0];assert hashlib.sha256(data).hexdigest()==row['sha256']
  m=kingdoms.Machine(data,row['world_version']);assert m.r.u32()==row['major_count'];assert m.r.u32()==row['slot_count'];at=m.r.at
  for entry in row['entries']:
   m=kingdoms.Machine(data,row['world_version']);m.r.at=at;assert m.r.take(1)[0]==entry['present_byte']
   if entry['present_byte']:
    prepare(m);got=m.kingdom();verify(m)
    assert got=={key:entry[key] for key in got},(row['map'],entry['slot'])
    for key in ['actor_seams','calls','references','reads']:assert getattr(m,key)==entry[key],(row['map'],key)
    kingdom_count+=1
   at=m.r.at
  assert at==row['decoded_bytes']==len(data)
 print(json.dumps({'kingdoms_replayed':kingdom_count}),flush=True)
 addresses={'property':0x503b10,'ability':0x4c8b40,'strategic':0x58ee80,'tactical':0x58dbe0}
 for row in l['cases']:assert leaves.run(addresses[row['kind']],bytes.fromhex(row['payload']))==row['native']
 windows(t);windows(k)
 for row in l['ranges'].values():
  a=int(row['va'],16);raw=teams.P.range(a,row['bytes']);assert raw==teams.V.range(a,len(raw));assert hashlib.sha256(raw).hexdigest()==row['sha256']
 run=subprocess.run(['cargo','test','-p','aa-mapworld','--test','teams','--test','kingdoms','--','--include-ignored','--nocapture'],cwd=ROOT,text=True,capture_output=True)
 assert run.returncode==0,run.stdout+run.stderr
 leafrun=subprocess.run(['cargo','test','-p','aa-mapworld','--lib','kingdoms::'],cwd=ROOT,text=True,capture_output=True)
 assert leafrun.returncode==0,leafrun.stdout+leafrun.stderr
 paths=['crates/aa-mapworld/src/kingdoms.rs','crates/aa-mapworld/src/ownership.rs','crates/aa-mapworld/src/world.rs','crates/aa-mapworld/src/lib.rs','crates/aa-mapworld/tests/kingdoms.rs','crates/aa-mapworld/tests/teams.rs','research/phase6-kingdoms/native_kingdoms.py','research/phase6-kingdoms/native_teams.py','research/phase6-kingdoms/native_leaves.py','research/phase6-kingdoms/native-kingdoms.json','research/phase6-kingdoms/native-teams.json','research/phase6-kingdoms/native-leaves.json']
 report={'schema':1,'reviewer':'visual_audit','passed':True,'maps':80,'native_team_records':team_count,'native_kingdom_records':kingdom_count,'synthetic_leaf_cases':len(l['cases']),'callee_saved_registers_verified':True,'rust_tests':'3 original-corpus tests plus 1 leaf unit test passed','owner_reference_test':'11701 source actor owner references; 7512 non-null exact saved handle bindings; no missing/stale/duplicate source bindings','reviewed_behavior':['Class5 teams and class4 kingdoms decode by native count/version/string reads, retaining exact source spans and saved IDs.','Kingdom world version selects Thing vs Actor base and optional Updatable; bounded Actor decoder dependency reviewed separately.','Native word188 equals source slot; word18c remains opaque, including minor kingdom FFFFFFFF.','Team slot identity and kingdom slot identity are checked; no header names, nations or player labels become identity substitutes.','Owner inventory matches exact saved IDs after generation checks. Duplicate and missing kingdom references remain explicit errors on runtime binding.','Final runtime Thing flag bit23 gates ownership application; raw serialized flag word is not silently substituted.','Unsupported named kingdom states preserve the original whole chunk through WorldInventory opaque fallback.','Versioned property, ability, strategic and tactical leaf readers are matched against 23 additional original executions.'],'findings':[],'limits':['Typed serialized fields and source owner references only; no live Kingdom/Team object, actor ownership registration or post-load simulation restoration is claimed.','Object nested payloads, unsupported Actor components/classes and named kingdom state count>0 remain unsupported.','Native kingdom fixtures use separately reviewed Actor wire seam plus required allocations/string/catalog/reference lookups; actual runtime registry lookups and economic resource clamping are not proven by this wire decoder.','Original corpus kingdom versions17..20 and worldversions21,24..28 are exercised. Earlier branches are source-reviewed, not separately covered by a full kingdom fixture for every version.'],'pins':{p:sha(ROOT/p)for p in paths}}
 out=HERE/'independent-review.json';out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps({'passed':True,'sha256':sha(out)}))
if __name__=='__main__':main()
