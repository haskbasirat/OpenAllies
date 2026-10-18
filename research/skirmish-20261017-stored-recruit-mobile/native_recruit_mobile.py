"""Whole581F30 Recruit and5757C0 MobileDeployment InitExisting, actual
4D1050/529C30 getters, real primary/Updatable vtable words, original/v14
allPC. Only50ACC0 is a named other-owner callback (required failures and
mutations retained); no scheduler success, wholeActor or World claim.
"""
from pathlib import Path
import sys,json,hashlib,gc,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
class Required(Exception):pass
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 #Selected Actor source constructor554: Object49C760 is an explicit
 #completed-base transport; all Actor field stores execute natively.
 m.thiscall(0x4013e0,0xb43b40,[]);definition_ctor=m.alloc(0x700);m.u.mem_write(definition_ctor,b'\xa5'*0x700)
 name=m.alloc(4);m.w(name,m.cstring('actor'));category=m.alloc(32)
 m.stubs[0x49c760]=lambda mm,u:mm.ret(u.reg_read(UC_X86_REG_ECX),pop=8)
 m.thiscall(0x4624b0,definition_ctor,[name,category]);del m.stubs[0x49c760]
 source554_constructor=m.r(definition_ctor+0x554)
 a=m.alloc(0x180);d=m.alloc(0x700);c=m.alloc(0x20);g=m.alloc(0x300);m.w(0xb36534,g);m.w(c+4,a);m.w(a+4,d)
 vtables={'recruit':m.r(0xa3e6d0+8),'research':m.r(0xa3eb58+8),'mobile':m.r(0xa3cd00+8),'mobile_updatable_gate':m.r(0xa3ccd8+12)}
 assert vtables==dict(recruit=0x581f30,research=0x6a71d0,mobile=0x5757c0,mobile_updatable_gate=0x6a71d0)
 assert m.thiscall(vtables['mobile_updatable_gate'],c+8,[])['eax']&255==1
 recruit=[]
 for targets in [[],[0],[1],[0,1],[1,0],[0x80000000,0,1]]:
  for prior in [[0,0],[128,255]]:
   head=0
   for raw in reversed(targets):
    target=m.alloc(0x200);m.w(target+0x184,raw);link=m.alloc(8);m.w(link,target);m.w(link+4,head);head=link
   m.w(d+0x4ec,head);m.u.mem_write(c+0x10,bytes(prior));m.w(c,0xa3e6d0);writes=[]
   def observe(u,at,size,_):
    if at in [0x581f50,0x581f55]:writes.append(0x10 if at==0x581f50 else 0x11)
   hook=m.u.hook_add(UC_HOOK_CODE,observe);out=m.thiscall(vtables['recruit'],c,[]);m.u.hook_del(hook)
   recruit.append(dict(targets=targets,prior=prior,bytes=list(m.u.mem_read(c+0x10,2)),writes=writes,result=out['eax']&255))
 mobile=[]
 for low,high in [(0,0),(1<<26,0),(0,1<<4),(0xffffffff,0xffffffff)]:
  for delay in [0,0x38d1b717,0x3f800000,0xbf800000,0x4b800001]:
   for mode in ['ordinary','required','mutate']:
    m.w(c,0xa3cd00);m.w(c+8,0xa3ccd8);m.w(c+0x14,0xbf800000);m.w(a+0x100,low);m.w(a+0x104,high);m.w(d+0x554,delay);m.w(g+0xe0,0x42160000);calls=[];status='ok'
    def schedule(mm,u):
     s=u.reg_read(UC_X86_REG_ESP);assert u.reg_read(UC_X86_REG_ECX)==c+8
     calls.append(dict(delay=mm.r(s+4),data=mm.r(s+8)))
     if mode=='required':raise Required('queue')
     if mode=='mutate':mm.w(g+0xe0,0x40600000);mm.w(d+0x554,0x40000000)
     mm.ret(pop=8)
    m.stubs[0x50acc0]=schedule
    try:out=m.thiscall(vtables['mobile'],c,[]);result=out['eax']&255
    except Required:status='required:queue';result=None
    mobile.append(dict(low=low,high=high,delay=delay,mode=mode,initial_begin=0x42160000,begin=m.r(g+0xe0),definition_delay=m.r(d+0x554),calls=calls,word14=m.r(c+0x14),status=status,result=result))
 return dict(source554_constructor=source554_constructor,unpacking_source_name=m.raw_wstr(0xa06858),vtables=vtables,recruit=recruit,mobile=mobile),pins,seams
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,output=a));pins.update(pa);seams|=sa|sb
 doc=dict(scope=__doc__,rows=rows,executed_pins={hex(a):b for a,b in sorted(pins.items())},intercepted_not_executed=[hex(a)for a in sorted(seams)],images={k:hashlib.sha256(v.path.read_bytes()).hexdigest()for k,v in [('original',h.P),('v14',h.V)]},producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 for a,b in pins.items():assert bytes.fromhex(b)==h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
 p=HERE/'native-recruit-mobile.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n',encoding='utf8',newline='\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=3,machines=6,recruit_runs=2*sum(len(r['output']['recruit'])for r in rows),mobile_runs=2*sum(len(r['output']['mobile'])for r in rows),pins=len(pins))))
if __name__=='__main__':main()
