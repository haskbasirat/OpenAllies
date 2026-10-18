"""Unchanged4F0A70->553BA0 on captured actual first-Kingdom direct-child probes.
The diagnostic calls are not naturally reached by that Kingdom's Init. Actual
captured grid/source/query fields are used; only tile0 is admitted to native
reads (the rest of allocated tile storage is explicitly outside this probe).
No collision/query/source callback is replaced. The descriptor constructor's
orientation source is an explicit probe and gets overwritten before use.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement')); import harness as h
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ
ORIGINAL=h.P
H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image):
 h.P=ORIGINAL if image=='original' else h.V; m=h.Machine(c['precision'])
 world=m.alloc(0x100);gmap=m.alloc(0x100);definition=m.alloc(0x600);actor=m.alloc(0x200);query=m.alloc(0x80)
 n=c['grid']['dimensions'][0]*c['grid']['dimensions'][1];tiles=m.alloc(n*48)
 m.w32(0xb36534,world);m.w32(world+0x30,gmap)
 m.w32(gmap+4,c['grid']['coordinate_shift'],c['grid']['cell_size_bits'],*c['extents'])
 m.w32(gmap+0x1c,*c['grid']['dimensions']);m.w32(gmap+0x28,tiles);m.w32(tiles+0x20,c['tile0_flags'])
 m.call(0x46eb60,gmap+0x30,[c['grid']['cell_size_bits']]); assert m.fpu_top()==0
 #552DD0's actual constructor dependencies are explicit transport probes; all
 #orientation/shape/position words used by this operation are overwritten.
 axes=m.alloc(0x100);vectors=m.alloc(0x40);m.w32(0xb39990,axes);m.w32(axes+0x64,vectors);m.w32(vectors+4,0,0xbf800000,0)
 m.w32(0xb3a8c4,0,0,0,0,0,0)
 m.call(0x552dd0,query,[]);assert m.fpu_top()==0
 m.w32(definition,0xa05540);m.w32(definition+0x1c,c['definition_flags']);m.w32(definition+0x38,*c['shape'])
 m.w32(actor,0xa299d8,definition);m.w32(actor+0x14,c['owner'])
 m.w32(query,*c['shape']);m.w32(query+0x18,definition,0,actor,0,0,0,0,0,c['required_actor_flags'],c['mask'])
 m.w32(query+0x40,query,*c['facing'],*c['position'],0)
 calls=[]
 def code(u,a,n,_):
  if a in (0x553ba0,0x4f0a8b):calls.append(hex(a))
  assert a not in (0x4d5600,0x4cede0,0x4e52a0),hex(a)
 def read(u,access,address,size,value,_):
  if tiles+48<=address<tiles+n*48:raise AssertionError('unexpected nonzero tile read '+hex(address))
 m.u.hook_add(UC_HOOK_CODE,code);m.u.hook_add(UC_HOOK_MEM_READ,read)
 before=bytes(m.u.mem_read(query,0x58)); result=m.call(0x4f0a70,world,[query,definition])
 assert calls==['0x553ba0','0x4f0a8b'],calls
 assert m.final_esp==h.STACK+12; assert m.fpu_top()==0
 assert bytes(m.u.mem_read(query,0x58))==before
 return {'result':result,'calls':calls,'descriptor_unchanged':True,'no_native_membership_nodes':m.r32(tiles+8)==0}
def main():
 p=HERE/'actual-direct-child.json';capture=json.loads(p.read_text());rows=[]
 for c in capture['cases']:
  a=run(c,'original');b=run(c,'v14');assert a==b; assert a['result']==c['result']; rows.append({'input':c,'output':a})
 result={'schema':1,'scope':__doc__,'capture_sha256':H(p.read_bytes()),'original_sha256':H(ORIGINAL.b),'v14_sha256':H(h.V.b),'cases':rows,'native_executions':len(rows)*2,'instructions':[{'address':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())]}
 out=HERE/'native-actual-direct-child.json'
 if '--write' in sys.argv:out.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(out.read_text())==result
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'pins':len(h.VISITED),'sha256':H(out.read_bytes())}))
if __name__=='__main__':main()
