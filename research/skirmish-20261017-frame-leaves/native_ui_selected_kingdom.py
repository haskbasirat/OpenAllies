"""Actual76C2B0 current Session68 selection and complete4F48E0 lookup.
The current GameSettings/World/Kingdom/CString records are explicit synthetic
inputs. Real4BEDF0,4012C0 refcount copy,96CB83 wide comparison and refcount
release execute. Only CString manager virtual+10 is a named transport seam
returning that same manager; no allocator/free branch is reached. Includes
the pure76AC90 getter on actual Session+F8 for the locked readonly UI lease.
No whole5C3000/camera/VWorld/resource completion is claimed.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
def string(m,units,manager):
 p=m.alloc(18+2*len(units));m.w32(p,manager,len(units),len(units),2)
 m.u.mem_write(p+16,b''.join(x.to_bytes(2,'little')for x in units)+b'\0\0')
 return p+16
def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 manager=m.alloc(4);vt=m.alloc(20);leaf=m.stub_next;m.stub_next+=16
 m.w32(manager,vt);m.w32(vt+16,leaf);m.seams[leaf]=lambda x:x.ret(value=x.ecx())
 session=m.alloc(0x100);settings=m.alloc(0x50);world=m.alloc(0x160)
 m.w32(session+0x68,settings);m.w32(0xb36534,world)
 m.w32(settings+0x3c,string(m,c['header'],manager))
 entries=m.alloc(12*len(c['entries']))
 for i,(flag,name)in enumerate(c['entries']):
  m.w8(entries+12*i+4,flag);m.w32(entries+12*i+8,string(m,name,manager))
 m.w32(settings+0x44,entries,len(c['entries']),len(c['entries']))
 slots=m.alloc(4*len(c['kingdoms']));kingdoms=[]
 for i,name in enumerate(c['kingdoms']):
  p=0 if name is None else m.alloc(0x20)
  if p:m.w32(p+0x18,string(m,name,manager))
  kingdoms.append(p);m.w32(slots+4*i,p)
 m.w32(world+0x148,slots,len(kingdoms),len(kingdoms))
 selected=next((name for flag,name in c['entries']if flag==0 and len(name)),None)if c['header']else None
 #Unreached GWorld is deliberately NULL. The harness rejects any ordinary
 #page-zero read, so the early-return paths cannot hide an eager World read.
 if selected is None:m.w32(0xb36534,0)
 result=m.call(0x76c2b0,session)
 chosen=None if result==0 else kingdoms.index(result)
 def nul(v):return v[:v.index(0)]if 0 in v else v
 expected=next((i for i,name in enumerate(c['kingdoms'])if name is not None and selected is not None and nul(name)==nul(selected)),None)
 assert chosen==expected,(c,chosen,expected)
 return {'slot':chosen}
def cases():
 K=list(map(ord,'K'));k=list(map(ord,'k'));Z=[ord('Z')]
 yield {'header':[],'entries':[[0,K]],'kingdoms':[K]}
 yield {'header':[0],'entries':[[0,K]],'kingdoms':[K]}
 yield {'header':K,'entries':[],'kingdoms':[K]}
 for flag in (0,1,128,255):
  yield {'header':K,'entries':[[flag,K],[0,k]],'kingdoms':[None,K,k]}
 for names in ([K,K],[k,K],[None,K],[],[K+[0]+Z,K]):
  for search in (K,K+[0]+Z,Z):
   yield {'header':K,'entries':[[1,Z],[0,[]],[0,search],[0,K]],'kingdoms':names}
def main():
 rows=[]
 for pc,c in itertools.product(h.PRECISION,cases()):
  a=run('original',pc,c);b=run('v14',pc,c);assert a==b
  rows.append({'precision':pc,'input':c,'output':a})
 getter=[]
 for image,pc,value in itertools.product(('original','v14'),h.PRECISION,(0,5,7,0xffffffff)):
  h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);p=m.alloc(0x100)
  m.w32(p+0xf8,value);assert m.call(0x76ac90,p)==value
  getter.append({'image':image,'precision':pc,'value':value})
 pins=[]
 for a,b in sorted(h.VISITED.items()):
  assert bytes.fromhex(b)==ORIGINAL.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
  pins.append({'va':hex(a),'bytes':b})
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
      'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':rows,'getter_cases':getter,'executed_pins':pins}
 path=HERE/'native-ui-selected-kingdom.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'selection_rows':len(rows),'selection_executions':2*len(rows),'getter_executions':len(getter),'pins':len(pins)}))
if __name__=='__main__':main()
