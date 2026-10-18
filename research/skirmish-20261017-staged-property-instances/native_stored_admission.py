"""Unchanged5035E0/503390/4CEF80/581280/5032D0/45EEF0 on captured live owners.

Actual member/group/parameter arrays, source masks/property lists and current
instance definitions are inputs. No property decision or membership seam.
Every modeled allocation's other bytes are poisoned and guarded against reads.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_READ

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 tracked=[];allowed=[]
 def alloc(size):
  q=m.alloc(size);m.u.mem_write(q,b'\xcd'*size);tracked.append((q,q+size));return q
 def word(at,value):m.w(at,value);allowed.append((at,at+4))
 def byte(at,value):m.wb(at,value);allowed.append((at,at+1))
 def array(values):
  q=alloc(max(4,4*len(values)))
  for index,value in enumerate(values):word(q+4*index,value)
  return q
 def linked(values):
  head=0
  for value in reversed(values):q=alloc(8);word(q,value);word(q+4,head);head=q
  return head
 ids=set(c['requires'])
 for a in c['actors'].values():ids.update(v['definition']for v in a['available'])
 for d in c['definitions'].values():ids.update(d['available'])
 properties={i:alloc(0x40)for i in ids};requested=alloc(0x40)
 word(requested+0x38,linked([properties[i]for i in c['requires']]))
 actors={int(key):alloc(0x200)for key in c['actors']};definitions={key:alloc(0x200)for key in c['definitions']}
 for key,d in c['definitions'].items():
  q=definitions[key];word(q+0x174,d['mask']);word(q+0x1d4,linked([properties[i]for i in d['available']]))
 for key,a in c['actors'].items():
  q=actors[int(key)]
  if a['properties']:
   p=alloc(0xe0);word(q+0x78,p);instances=[]
   for value in a['available']:
    instance=alloc(0x18);word(instance+0xc,properties[value['definition']]);byte(instance+0x10,int(value['disabled']));instances.append(instance)
   word(p+0x10,array(instances));word(p+0x14,len(instances))
  else:word(q+0x78,0)
  o=a['organization']
  if o is None:word(q+0x7c,0)
  else:
   org=alloc(0x30);layout=alloc(0x2c);word(q+0x7c,org)
   word(org+0x28,array([actors[i]if i else 0 for i in o['members']]));word(org+0x2c,len(o['members']))
   word(org+0x18,layout);word(layout+0x28,array(o['groups']));word(org+0x1c,array([definitions[i]if i else 0 for i in o['parameters']]))
 events=[];actor_ids={v:k for k,v in actors.items()};definition_ids={v:k for k,v in definitions.items()}
 m.observe(0x5035e0,lambda mm:events.append(['actor',actor_ids[mm.arg(1)]]))
 m.observe(0x503390,lambda mm:events.append(['definition',definition_ids[mm.arg(1)]]))
 def read(u,access,address,size,value,user):
  if any(address<end and address+size>begin for begin,end in tracked):
   assert any(begin<=address and address+size<=end for begin,end in allowed),('unbound native field read',hex(address),size)
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 result=bool(m.call(0x5035e0,0,[actors[c['root']],requested],pop=0)&255)
 return dict(result=result,events=events),m.visited

def main():
 doc=json.loads((HERE/'admission-actual-inputs.json').read_text());rows=[];pins={};runs=0
 for row in doc['rows']:
  bypc={}
  for pc in nm.PRECISIONS:
   a,pa=run(row['input'],'original',pc);b,pb=run(row['input'],'v14',pc);assert a==b;runs+=2
   assert a['result']==row['result'],(row['phase'],a,row['result']);bypc[pc]=a;pins.update(pa);pins.update(pb)
  rows.append(dict(phase=row['phase'],input=row['input'],result=row['result'],native=bypc))
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 result=dict(schema=1,scope=__doc__,rows=rows,native_executions=runs,instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 path=HERE/'native-stored-admission.json'
 if '--write'in sys.argv:path.write_text(json.dumps(result,indent=2)+'\n')
 else:assert json.loads(path.read_text())==result
 print(json.dumps(dict(rows=len(rows),runs=runs,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
