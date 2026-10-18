"""Reverse-order independent native header replay and stored-field checks."""
from pathlib import Path
import hashlib,json,struct,time
import native_header as native
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()

def text(machine,field):
 p=machine.read(field);n=machine.read(p-12)
 return bytes(machine.u.mem_read(p,n*2)).decode('utf16') if n else ''

def main():
 proof_path=HERE/'native-header.json';proof_bytes=proof_path.read_bytes();proof=json.loads(proof_bytes)
 results=[];started=time.monotonic()
 for row in reversed(proof['maps']):
  p=Path(row['path']);data=native.read_chunks(p)['MAP_HEADER'][0]
  assert sha(data)==row['payload_sha256'],p
  raw=native.decode(data);m=native.Machine(data);actual=m.run()
  for key in ['prefix_bytes','decoded_bytes','display_extents_bits','creator_size_bits','rng_seed','reads','seams']:
   assert actual[key]==row[key],(p,key)
  obj=m.object
  assert [m.read(obj+o)for o in [0xc,0x10]]==raw['words0c10'],p
  assert [m.read(obj+o)for o in [0x18,0x1c]]==raw['words181c'],p
  assert text(m,obj+0x20)==raw['labels'][0][0],p
  assert text(m,obj+0x28)==raw['labels'][1][0],p
  assert [m.read(obj+o)for o in [0x50,0x54]]==raw['metadata'][:2],p
  assert [text(m,obj+o)for o in [0x58,0x5c,0x60]]==raw['metadata'][2:5],p
  assert bytes(m.u.mem_read(obj+0x64,1))[0]==raw['metadata'][5],p
  assert [text(m,obj+o)for o in [0x38,0x3c]]==raw['paths'],p
  if raw['version']>=6:assert m.read(obj+0x40)==raw['word40'],p
  # Native WorldParameters team/kingdom storage counts and identity strings.
  parameters=m.params
  assert m.read(parameters+8)==len(raw['parameters']['teams'])
  team_array=m.read(parameters+4)
  for i,t in enumerate(raw['parameters']['teams']):
   assert text(m,team_array+i*12)==t[0],p
   assert text(m,team_array+i*12+4)==t[1][0],p
  assert m.read(parameters+0x14)==len(raw['parameters']['kingdoms'])
  kingdoms=m.read(parameters+0x10)
  for i,k in enumerate(raw['parameters']['kingdoms']):
   base=kingdoms+i*0x6c
   assert text(m,base)==k['name'],p
   assert text(m,base+4)==k['label'][0],p
   for key,offset in [('string0c',0xc),('string2c',0x2c),('string14',0x14),('string58',0x58)]:
    if k[key]is not None:assert text(m,base+offset)==k[key],(p,key)
   for key,offset in [('byte10',0x10),('byte29',0x29),('byte28',0x28),('byte31',0x31),('byte32',0x32)]:
    assert bytes(m.u.mem_read(base+offset,1))[0]==k[key],(p,key)
  results.append({'path':str(p),'payload_sha256':sha(data),'version':raw['version'],'teams':len(raw['parameters']['teams']),'kingdoms':len(raw['parameters']['kingdoms']),'passed':True})
 assert len(results)==80 and proof_path.read_bytes()==proof_bytes
 report={'schema':1,'passed':True,'reviewer':'terrain_roads','maps':results,'seconds':time.monotonic()-started,'original_fixture_sha256':sha(proof_bytes),
         'source_sha256':{p.relative_to(ROOT).as_posix():sha(p.read_bytes())for p in [ROOT/'crates/aa-mapworld/src/header.rs',ROOT/'crates/aa-mapworld/tests/header.rs',HERE/'native_header.py',HERE/'inspect_header.py',Path(__file__)]},
         'method':'Original readers replayed in reverse map order; exact stream footprints/seams/extent/seed compared; independently checked native stored header metadata, team/kingdom counts, identities, localization keys and scalar flags against source payload layout.',
         'limits':['Native allocation/string storage/reference resolution are declared fixture seams.','WorldCreator class name is verified and its native reader invoked directly; generic RTTI object allocation/dispatch is not executed.','Runtime definition resolution, applied rules, kingdom/owner creation and localized display text are not claimed.']}
 (HERE/'independent-replay.json').write_text(json.dumps(report,indent=2));print(json.dumps({'passed':True,'maps':80,'seconds':report['seconds'],'sha256':sha((HERE/'independent-replay.json').read_bytes())}))
if __name__=='__main__':main()
