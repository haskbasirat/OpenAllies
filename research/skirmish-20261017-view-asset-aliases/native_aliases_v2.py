"""Actual401BD0 AVar registration then real AVars*.tgi source loading.
Bounded23 VWorld asset alias Vars: native constructor absence, sourceString
creation/overrides and counted410CB0 copy. No profile/console overrides,
missing-name fallback or complete application/other AVar effects certified.
"""
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
order=full.order
KEYS=['TextureCircle','TextureSquare','TextureHollowCircle','TextureHollowSquare','VFXMoveCommand','VFXSelectFacing','VFXHealthBar','VFXMoraleBar','VFXSupplyStockpileBar','VFXAttackEfficiencyBar','VFXGhosting','VFXIllegalPlacement','VFXCircleBrush','VFXSquareBrush','VFXProjectedBoxTexture','VFXProjectedCircleTexture','VFXLocationMarkerEditor','VFXRecruitMarkerRallyPoint','VFXText','VFXTextUnscaled','VFXDamageNumbers','VFXHealNumbers','TextureEnvironmentMap']
def files():
 #Actual recovered original Data/ and v14 overlay wildcard corpus. Rust
 #independently compares these complete file identities to V14Sources catalog.
 overlay=GAME/'reconstruction/native-game/runtime-expanded-v14/Data'
 original=GAME/'reconstruction/assets/original/Data'
 names=sorted({p.name for folder in[original,overlay]for p in folder.glob('AVars*.tgi')},key=str.casefold)
 assert names==['AVars.tgi','AVars_AA.tgi','AVars_ClientCadence.tgi','AVars_Locale.tgi','AVars_version.tgi'],names
 for name in names:
  path=overlay/name
  if not path.is_file():path=original/name
  yield name,path.read_bytes()

def run(image,pc):
 order.T.P=image;m=order.Machine(b'',pc);pins={};seams=set()
 def executed(u,at,size,_):
  if order.ORIGINAL.base<=at<order.ORIGINAL.base+order.ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed)
 m.invoke(0x4013e0,0xb43b40,[]);settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[]);
 try:m.invoke(0x401bd0,settings,[])
 except Exception:
  print('recent',[hex(p)for p in m.recent]);raise

 initial=full.snapshot(m,settings);assert not any(r['name']in KEYS for r in initial)
 stages=[]
 for name,raw in files():
  m.lines=order.parser.lines_of(raw);m.at=0;m.thrown=None;m.messages=[];parsed=m.parse();assert parsed['error']is None,parsed
  root=m.parsed_root
  for i in range(m.r(root+0x38)):
   node=m.r(m.r(root+0x34)+4*i)
   if m.text(m.r(node)).lower()=='vars':m.invoke(0x4b3c10,settings,[node])
  stages.append(dict(file=name,sha256=hashlib.sha256(raw).hexdigest(),aliases=[r for r in full.snapshot(m,settings)if r['name']in KEYS]))
 final=stages[-1]['aliases'];assert len(final)==23 and all(r['kind']==4 and r['flags']==0 for r in final)
 copies=[]
 for key in KEYS:
  out=m.alloc(4);literal=m.string(list(map(ord,key)));m.invoke(0x410cb0,settings,[out,literal]);value=m.text(m.r(out));assert value==next(r['value']for r in final if r['name']==key);copies.append(dict(name=key,value=value));m.invoke(0x4047a0,out,[])
 #A real current String update keeps an already returned CString copy alive.
 key='TextureCircle';var=next(m.r(m.r(settings+8)+i*4)for i in range(m.r(settings+12))if m.text(m.r(m.r(m.r(settings+8)+i*4)+0x1c))==key)
 old=m.r(var+0x10);before=m.r(old-4);out=m.alloc(4);m.invoke(0x410cb0,settings,[out,m.string(list(map(ord,key)))]);copied=m.r(old-4)
 replacement=[ord('Q'),0,0xd800];text=m.string(replacement);m.invoke(0x4b3a30,var,[text]);after_set=m.r(old-4)
 old_value=m.text(m.r(out));new=m.r(var+0x10);length=m.r(new-12);new_value=[int.from_bytes(m.u.mem_read(new+2*i,2),'little')for i in range(length)]
 m.invoke(0x4047a0,out,[]);after_drop=m.r(old-4)
 assert copied==before+1 and after_set==before and after_drop==before-1
 assert old_value==next(r['value']for r in final if r['name']==key)and new_value==replacement
 return dict(constructor_count=len(initial),constructor_names=[r['name']for r in initial],stages=stages,copies=copies,lifetime=dict(key=key,before=before,copied=copied,after_set=after_set,after_drop=after_drop,old_value=old_value,new_value=new_value)),pins,seams

def main():
 out=[];pins_a={};pins_b={};seams=set()
 for pc in(['Double53']if '--probe'in sys.argv else order.PCS):
  a,pa,sa=run(order.ORIGINAL,pc);b,pb,sb=run(order.EXPANDED,pc)
  assert a==b, 'selected logical results differ'
  out.append(dict(precision=pc,native=a));pins_a.update(pa);pins_b.update(pb);seams|=sa|sb
  print(pc,'native125 ctor names +23 aliases/lifetime passed',flush=True)
 for image,pins in [(order.ORIGINAL,pins_a),(order.EXPANDED,pins_b)]:
  for at,raw in pins.items():assert image.range(at,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
 assert pins_a.keys()==pins_b.keys()
 differences={hex(at):dict(original=pins_a[at],expanded=pins_b[at])for at in pins_a if pins_a[at]!=pins_b[at]}
 #The installed v14 changes an unrelated constructor float immediate. We do
 #not claim all125 initial current values equal, only names and23 absent keys.
 assert differences=={'0x402446':dict(original='683333b33f',expanded='6866660640')},differences
 doc=dict(schema=1,scope=__doc__,rows=out,executed_original={hex(a):b for a,b in sorted(pins_a.items())},executed_expanded={hex(a):b for a,b in sorted(pins_b.items())},instruction_differences=differences,intercepted=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 if '--probe'not in sys.argv:
  p=HERE/'native-aliases-v2.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
  if '--write'in sys.argv:p.write_bytes(raw)
  else:assert p.read_bytes()==raw
  print(json.dumps(dict(rows=len(out),machines=2*len(out),constructors=out[0]['native']['constructor_count'],aliases=len(out[0]['native']['copies']),pins_per_image=len(pins_a),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
