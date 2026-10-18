"""Complete original4D8CF0 saved Actor layer through its final return for all source Kingdoms taking the bit29/null-owner/empty-name path. Thing/Object read is a prior-phase seam; real requirement, Properties load, scalar/list reads and post-tail branches execute. Definition numbering flag tested both ways. Constructor word15C retains its separately proven FFFFFFFF default for modern source versions."""
from pathlib import Path
import sys,json,struct,hashlib
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase6-actor-payloads'));sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_actor_payload import TailMachine,P,V
from native_header import read_chunks
from unicorn.x86_const import *
rows=[];executed={}
for source in json.load(open(ROOT/'research/phase6-kingdoms/native-kingdoms.json'))['maps']:
 chunk=read_chunks(Path(source['map']))['KINGDOMS'][0]
 for entry in source['entries']:
  if not entry.get('actor_seams'):continue
  seam=entry['actor_seams'][0];version=seam['actor_version'];start=seam['tail_start']-(16 if version>=9 else 12);end=seam['end']
  raw=chunk[start:end]
  for numbered in (0,1):
   m=TailMachine(raw);m.w(m.thing+0x15c,0xffffffff);m.w(m.thing+8,1<<29);m.w(m.thing,0xa299d8);m.u.mem_write(m.definition+0x1b0,bytes([numbered]));m.w(0x2088000,65535)
   native=m.run_actor(version,64);m.u.emu_start(0x4d9714,m.stop,count=100000)
   assert m.u.reg_read(UC_X86_REG_EIP)==m.stop and m.at==len(raw)
   assert m.r(m.thing+0xe8)==0 and m.r(m.thing+0x158)==0
   assert m.r(m.r(m.thing+0xdc)-12)==0
   executed.update(m.executed)
   rows.append({'map':str(Path(source['map']).relative_to(ROOT.parent)),'slot':entry['slot'],'serialized_id':entry['serialized_id'],'source_chunk_sha256':hashlib.sha256(chunk).hexdigest(),'actor_span':[start,len(raw)],'actor_hex':raw.hex(),'numbered':numbered,'native':native})
pins=[]
for a,b in sorted(executed.items()):assert b==V.range(a,len(b));pins.append({'address':hex(a),'bytes':b.hex()})
out={'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'instructions':pins}
p=Path(__file__).with_name('native-kingdom-actor-load.json');p.write_text(json.dumps(out,indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
