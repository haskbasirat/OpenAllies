"""Exact bounded source-completion context/list proof; no source arithmetic or OS FP claim."""
from pathlib import Path
import sys,json,struct
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified,sha,STACK,PRECISIONS
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
HERE=Path(__file__).resolve().parent

def remap(m,image):
 old=(original.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
 if new>old:m.u.mem_map(original.base+old,new-old)
 for _,rva,size,off in image.sections:m.u.mem_write(image.base+rva,image.b[off:off+size])

def linked(m,head):
 out=[];p=m.r(head)
 while p:
  out.append(m.r(p));assert len(out)<100
  assert m.r(m.r(p+4))==p
  p=m.r(p+8)
 return out

def main():
 rows=[];sequences=[];csv=[];pins={}
 for image_name,image in [('original',original),('v14',modified)]:
  for precision in PRECISIONS:
   for entries in [[],[(2,2),(20,2),(1,1),(21,2)],[(20,2),(20,2),(2,2),(0,0),(22,2)]]:
    m=Machine(precision);remap(m,image)
    manager=m.alloc(0x100);categories=m.alloc(0x100);kinds=m.alloc(0x100)
    m.w(manager+0x5c,categories);m.w(manager+0x68,kinds)
    m.seam(0x6f7810,lambda z:z.ret(z.alloc(z.arg(1))))
    registered=[]
    for ordinal,(category,kind) in enumerate(entries):
     d=m.alloc(0x20);m.w(d+0xc,category);m.w(d+0x18,kind)
     actor=m.alloc(0x500);registered.append((actor,category,kind))
     m.u.reg_write(UC_X86_REG_ESP,STACK);m.w(STACK+0x10,manager)
     m.u.reg_write(UC_X86_REG_EDI,manager);m.u.reg_write(UC_X86_REG_EBX,d);m.u.reg_write(UC_X86_REG_ESI,actor)
     m.u.emu_start(0x4bcf6b,0x4bcfc5,count=1000)
    actual=linked(m,kinds+8);expected=[a for a,c,k in reversed(registered) if k==2]
    assert actual==expected
    positions={a:i for i,(a,c,k) in enumerate(registered)}
    row={'image':image_name,'precision':precision,'registered_category_kind':entries,'actor_head_registration_ordinals':[positions[a] for a in actual]}
    rows.append(row)
    seen=[]
    addresses=[0x4bb480,0x4bb130,0x4bb730,0x4bbf90,0x4bb590,0x45f7a0,0x4bd440,0x4bc7f0]
    for address in addresses:
     def hook(z,address=address):
      seen.append({'function':f'{address:x}','fpcw':z.u.reg_read(UC_X86_REG_FPCW),'actor_ordinal':positions.get(z.ecx())});z.ret()
     m.seam(address,hook)
    m.call(0x4bd800,ecx=manager)
    assert [x['function'] for x in seen]==[f'{x:x}' for x in addresses[:5]]+['45f7a0']*len(actual)+[f'{x:x}' for x in addresses[-2:]]
    assert all(x['fpcw']==PRECISIONS[precision] for x in seen)
    assert [x['actor_ordinal'] for x in seen if x['function']=='45f7a0']==row['actor_head_registration_ordinals']
    sequences.append({'image':image_name,'precision':precision,'events':seen})
    assert not m.null_accesses;m.verify_v14();pins.update(m.visited)
   # Native empty-CSV guard and cache, twice on same actual variable identity.
   m=Machine(precision);remap(m,image);manager=m.alloc(0x100);kinds=m.alloc(0x100);m.w(manager+0x68,kinds)
   vt=m.alloc(0x40);m.w(manager+0x4c,vt);query=m.stub(lambda z:z.ret(0));m.w(vt+0x14,query)
   cell=m.alloc(0x30);s=m.alloc(0x20);m.w(s+4,0);m.w(cell+0x10,s+0x10)
   events=[]
   def lookup(z):
    p=z.arg(1);s=[]
    while (c:=struct.unpack('<H',z.u.mem_read(p,2))[0]):s.append(chr(c));p+=2
    events.append({'manager':f'{z.ecx():x}','name':''.join(s),'kind':z.arg(2)});z.ret(cell,pop=8)
   m.seam(0x4b2e60,lookup);m.w(0xb3b4f4,0)
   for _ in range(2):m.call(0x4bb590,ecx=manager)
   assert events==[{'manager':'b39938','name':'CombatValueCSV','kind':4}]
   csv.append({'image':image_name,'precision':precision,'calls':2,'lookup_events':events,'guard':m.r(0xb3b4f4),'cached_identity_matches':m.r(0xb3b4f0)==cell})
   assert not m.null_accesses;m.verify_v14();pins.update(m.visited)
 ranges=[('renderer_before_data',0x41a1ed,0x41a205),('game_data_on_app_init',0x41a55b,0x41a566),('completion_on_data_load',0x49051f,0x49052a),('completion_order',0x4bd800,0x4bd84e),('native_kind_head_prepend',0x4bcf6b,0x4bcfc5),('settlement_extent',0x45e6d0,0x45e6fc),('main_thread_net_start_later',0x41abc5,0x41abef),('renderer_behavior_flags',0x89198c,0x89199e)]
 static=[];cs=Cs(CS_ARCH_X86,CS_MODE_32)
 for name,a,b in ranges:
  raw=original.range(a,b-a);assert raw==modified.range(a,b-a)
  static.append({'name':name,'start':f'{a:x}','end':f'{b:x}','sha256':sha(raw),'bytes':raw.hex(),'instructions':[f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in cs.disasm(raw,a)]})
 d={'schema':1,'passed':True,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'list_cases':rows,'completion_cases':sequences,'csv_cases':csv,'static_windows':static,'instructions':[{'address':f'{a:x}','bytes':b.hex()} for a,b in sorted(pins.items())], 'scope':['Execute original4BCA00 list-insertion suffix after successful registry creation, over explicit already-created ActorData/category descriptors. Native allocation success is a seam. Full actor constructors/loaders are not executed.','Execute full4BD800 controller with all child operations as explicit seams: proves call/list order and inherited PC, not completed source arithmetic.','Execute full4BB590 empty-CSV/empty-Actor-list path twice: typed variable lookup and unopened stream query are leaves; cached variable identity is real.','Both PE images and all three PC settings; zero unguarded null accesses.'], 'precision_limit':'Main startup calls renderer before48DB60 and4BD800, before net thread starts. PC24 is statically expected from D3D9 default non-FPU-preserve behavior, not established by this oracle: D3D/OS is not run. Caller precision remains explicit; no restore-PC substitution or observed original source-load certificate is created.'}
 (HERE/'native-source-context.json').write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8')
 print('passed',len(rows),'list',len(sequences),'controller',len(csv),'CSV',len(pins),'pins')
if __name__=='__main__':main()
