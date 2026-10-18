"""Whole native4CF040/4E52D0/4E10F0 plus Actor138/140, no behavior seams.

Prior phase7 fixture supplies isolated input tables/questions only; every
output is freshly executed for each original/v14 image and all3 precisions.
Globals are explicit probe inputs, not fresh application values. This is not
Kingdom construction, live registry binding or global-writer provenance.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 ids={int(i):m.alloc(0x300)for i in list(c['teams'])+list(c['kingdoms'])}
 m.w(0xb28158,c['defaults'][0]);m.w(0xb2815c,c['defaults'][1])
 for key,row in c['teams'].items():
  p=ids[int(key)];m.w(p+0x38,row['slot']);m.w(p+0x4c,m.array(row['kingdoms']));m.w(p+0x58,m.array(row['teams']))
 for key,row in c['kingdoms'].items():
  p=ids[int(key)];m.w(p,0xa299d8);m.w(p+0x188,row['slot']);m.w(p+0x1ec,ids[row['team']]if row['team']is not None else 0)
  m.w(p+0x1d0,m.array(row['kingdoms']));m.w(p+0x1dc,m.array(row['teams']))
 actor=m.alloc(0x200);m.w(actor,0xa273f0)
 out=[]
 for q in c['questions']:
  other=q['other'];argument=ids[other]if other is not None else 0
  if q['kind']=='actor':
   owner=q['owner'];m.w(actor+0xe8,ids[owner]if owner is not None else 0)
   result=dict(relation=m.call(0x4cf040,actor,[argument]),accepts=bool(m.call(0x4d1eb0,actor,[argument])&255),enemy=bool(m.call(0x4e52a0,actor,[argument])&255))
  else:
   receiver=ids[q['id']];entry=0x4e52d0 if q['kind']=='kingdom'else 0x4e10f0
   result=dict(relation=m.call(entry,receiver,[argument]))
   if q['kind']=='kingdom':result.update(accepts=bool(m.call(0x4e5250,receiver,[argument])&255),enemy=bool(m.call(0x4e52a0,receiver,[argument])&255))
  out.append(dict(question=q,native=result))
 return out,m.visited
def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args()
 old=json.loads((ROOT/'research/phase7-runtime/native-relations.json').read_text())
 inputs=[]
 for c in old['cases']:
  inputs.append(dict(defaults=c['defaults'],teams=c['teams'],kingdoms=c['kingdoms'],questions=[{k:v for k,v in q.items()if k in ['kind','id','owner','other']}for q in c['queries']]))
 rows=[];pins={}
 for name,image in[('original',original),('v14',modified)]:
  for pc in['Single24','Double53','Extended64']:
   for c in inputs:
    out,visited=run(c,image,pc);pins.update(visited);rows.append(dict(image=name,precision=pc,input=c,queries=out))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,instructions=[])
 for address,raw in sorted(pins.items()):
  assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
  out['instructions'].append(dict(address=hex(address),bytes=raw.hex()))
 path=HERE/'native-zone-relations.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(rows=len(rows),queries=sum(len(r['queries'])for r in rows),pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
