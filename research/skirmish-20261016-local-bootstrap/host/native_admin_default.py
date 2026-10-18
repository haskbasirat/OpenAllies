"""Actual AdminTicksPerSecond registration callsite5069C8..5069FE,
real float2 constructor401710 and SettingsAdd4B3470. Eight preceding slots are
explicit fixture registrations (not a whole506010 construction claim).
Then real4B3C10 empty-source and profile(flags3/restricted) update paths run.
Successful allocator/string/debug-name services are inherited named leaves.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-combat-source-context'))
from native_context import Machine,P,V,PCS,LEAF_ENTRIES
from unicorn.x86_const import *
def words(m,v):
 return {'bits':m.r(v),'kind':m.r(v+0x24),'flags':m.r(v+0x28),'index':m.r(v+0x20)}
def main():
 rows=[];pins={}
 for image_name,image in [('original',P),('v14',V)]:
  for precision,cw in PCS.items():
   m=Machine(cw,image);settings=m.settings()
   for i in range(8):
    v=m.alloc(0x2c);m.invoke(0x401710,v,[m.string(list(map(ord,'explicit-prior-'+str(i)))),0x3f800000,0]);m.invoke(0x4b3470,settings,[i,v])
   m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ESI,settings);m.u.reg_write(UC_X86_REG_EDI,0xffffffff)
   m.u.emu_start(0x5069c8,0x5069fe,count=1000000);assert m.u.reg_read(UC_X86_REG_EIP)==0x5069fe
   assert m.r(settings+12)==9
   var=m.r(m.r(settings+8)+0x20);initial=words(m,var)
   text=m.r(var+0x1c);name=bytes(m.u.mem_read(text,m.r(text-12)*2)).decode('utf-16-le')
   assert name=='AdminTicksPerSecond' and initial=={'bits':0x41800000,'kind':2,'flags':0,'index':8},(name,initial)
   m.invoke(0x4b3c10,settings,[m.node('SVars')]);empty=words(m,var)
   m.w(settings+0x24,1);m.invoke(0x4b3c10,settings,[m.make('AdminTicksPerSecond','99',4)]);restricted=words(m,var)
   assert initial==empty==restricted
   assert m.r(m.r(settings+8)+0x20)==var
   session=m.alloc(0x178);node=m.alloc(0x2e8)
   m.w(session+0x24,0xa5a5a5a5);m.w(node+0x268,0xa5a5a5a5);m.w(node+0x280,0xa5a5a5a5)
   m.u.reg_write(UC_X86_REG_EBX,0xa5a5a5a5);m.u.reg_write(UC_X86_REG_ESI,session)
   m.u.emu_start(0x76f8c7,0x76f8c9,count=2);assert m.u.reg_read(UC_X86_REG_EBX)==0
   m.u.emu_start(0x76fae7,0x76faea,count=2)
   m.u.reg_write(UC_X86_REG_EBX,0xa5a5a5a5);m.u.reg_write(UC_X86_REG_ESI,node)
   m.u.emu_start(0x75ceb2,0x75ceb4,count=2);assert m.u.reg_read(UC_X86_REG_EBX)==0
   m.u.emu_start(0x75d074,0x75d07a,count=2);m.u.emu_start(0x75d0c9,0x75d0cf,count=2)
   ctor={'session_film_24':m.r(session+0x24)&255,'node_mirror_268':m.r(node+0x268),'node_ordering_client_280':m.r(node+0x280)}
   assert ctor=={'session_film_24':0,'node_mirror_268':0,'node_ordering_client_280':0}
   rows.append({'image':image_name,'precision':precision,'name':name,'initial':initial,'after_empty_source':empty,'after_restricted_profile':restricted,'same_identity':True,'constructor_fields':ctor})
   for a,n in m.visited.items():
    if P.base<=a<P.base+P.image_size and a not in m.seam_entries and a not in LEAF_ENTRIES:
     raw=P.range(a,n);assert raw==V.range(a,n);pins[a]=raw
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 report={'passed':True,'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'instructions':[{'address':hex(a),'bytes':raw.hex()}for a,raw in sorted(pins.items())]}
 (HERE/'native-admin-default.json').write_text(json.dumps(report,indent=2)+'\n');print('passed',len(rows),'registered default rows',len(pins),'native instruction pins')
if __name__=='__main__':main()
