"""Banner selected source46D630/46D370/46CEE0, real constructors/vector
stores and request46C7A0/46C830/46C8F0. Typed source getters, canonical
Texture factory, format/path and Name-atom transport are explicit services;
47D960 unrelated NamedData loader is a completed-base transport. No physical
Texture/GPU completion or whole global source registration order is implied.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
class Required(Exception):pass
NAMES=['NONE','EXPERIENCE','ENGAGED','ENTRENCHED','HERO','OUT_OF_SUPPLY','ROUT','UPKEEP_SHORTAGE','BANNER_ICON','BADGE']
def node(function='NONE',textures=None,decals=None,name='plane'):
 r=dict(function=function,flashing=False,name=name)
 if textures is not None:r['Texture']=[dict(texture=t) for t in textures]
 if decals is not None:r['Decal']=decals
 return r

def run(image,pc,case):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 sources={};gets=[];textures=[];requests=[];all_planes=[];all_decals=[]
 def register(n):
  p=m.alloc(4);sources[p]=dict(n)
  for k in ['Plane','Decal','Texture']:
   if k in n:sources[p][k]=[register(c) for c in n[k]]
  return p
 source=register(case['source'])
 def field(mm,u):
  p=u.reg_read(UC_X86_REG_ECX);key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));gets.append(key)
  cell=mm.alloc(4);mm.w(cell,mm.cstring(sources[p][key]));mm.ret(cell,pop=4)
 def count(mm,u):
  p=u.reg_read(UC_X86_REG_ECX);key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));gets.append('children:'+key);mm.ret(len(sources[p].get(key,[])),pop=4)
 def iterator(mm,u):
  it=u.reg_read(UC_X86_REG_ECX);p=mm.r(it);key=mm.raw_wstr(mm.r(it+8));mm.ret(sources[p][key][mm.r(it+4)])
 def boolean(mm,u):
  p=u.reg_read(UC_X86_REG_ECX);key=mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP)+4));gets.append(key);mm.ret(int(sources[p][key]),pop=4)
 def enum(mm,u):
  s=u.reg_read(UC_X86_REG_ESP);name=mm.wstr(mm.r(mm.r(s+4)));cell=mm.alloc(4);value=mm.alloc(4)
  if name.upper() not in NAMES:mm.ret(0,pop=4);return
  mm.w(value,NAMES.index(name.upper()));mm.w(cell,value);mm.ret(cell,pop=4)
 def format(mm,u):
  s=u.reg_read(UC_X86_REG_ESP);dst=mm.r(s+4);fmt=mm.wstr(mm.r(s+8));prefix=mm.wstr(mm.r(s+12));mm.w(dst,mm.cstring(fmt.replace('%s',prefix)));mm.ret(dst)
 def texture(mm,u):
  s=u.reg_read(UC_X86_REG_ESP);name=mm.wstr(mm.r(mm.r(s+4)));textures.append(name)
  if len(textures)-1==case.get('factory_stop',-1):raise Required('Texture')
  resource=mm.alloc(0x18);vt=mm.alloc(0x18);mm.w(resource,vt);logical=len(textures)
  def request(mm,u):
   s=u.reg_read(UC_X86_REG_ESP);requests.append([logical,mm.r(s+4),mm.r(s+8)])
   if len(requests)-1==case.get('request_stop',-1):raise Required('request')
   mm.ret(pop=8)
  mm.w(vt+0x10,mm.stub(request));mm.ret(resource)
 def name_atom(mm,u):
  s=u.reg_read(UC_X86_REG_ESP);dst=u.reg_read(UC_X86_REG_ECX);mm.w(dst,mm.cstring(mm.raw_wstr(mm.r(s+4))));mm.ret(dst,pop=4)
 def name_assign(mm,u):
  s=u.reg_read(UC_X86_REG_ESP);dst=u.reg_read(UC_X86_REG_ECX);mm.w(dst,mm.r(mm.r(s+4)));mm.ret(dst,pop=4)
 def diagnostic(mm,u):raise Required('diagnostic')
 m.stubs.update({0x5b54d0:field,0x5b4220:count,0x42be10:iterator,0x5b5380:boolean,0x497220:enum,0x405d60:format,
  0x4763c0:lambda mm,u:mm.ret(pop=4),0x5a77e0:texture,0x47d960:lambda mm,u:mm.ret(pop=4),0x454810:name_atom,0x4615e0:name_assign,0x410c40:diagnostic})
 def plane_observer(u,at,size,_):all_planes.append(u.reg_read(UC_X86_REG_ECX))
 def decal_observer(u,at,size,_):all_decals.append(u.reg_read(UC_X86_REG_ECX))
 m.u.hook_add(UC_HOOK_CODE,plane_observer,begin=0x46d370,end=0x46d370);m.u.hook_add(UC_HOOK_CODE,decal_observer,begin=0x46cee0,end=0x46cee0)
 data=m.alloc(0x40);ids=m.alloc(4);m.w(ids,m.cstring('banner'));m.thiscall(0x46cab0,data,[ids])
 status='ok'
 try:m.thiscall(0x46d630,data,[source])
 except Required as e:status='required:'+str(e)
 def decal(p):return dict(function=m.r(p+4),flashing=bytes(m.u.mem_read(p+8,1))[0],textures=m.r(p+0x10))
 def plane(p):return dict(function=m.r(p+4),flashing=bytes(m.u.mem_read(p+8,1))[0],name=m.wstr(m.r(p+0xc)),decals=[decal(m.r(m.r(p+0x10)+4*i)) for i in range(m.r(p+0x14))])
 out=dict(status=status,model=m.wstr(m.r(data+0x1c)),skin=m.r(data+0x20)!=0,
  planes=[plane(m.r(m.r(data+0x24)+4*i)) for i in range(m.r(data+0x28))],
  allocated_planes=[plane(p)for p in all_planes],allocated_decals=[decal(p)for p in all_decals],textures=textures,gets=gets,requests=requests)
 if status=='ok':
  try:m.thiscall(0x46c7a0,data,[case.get('delta',1),case.get('mode',0)])
  except Required as e:out['status']='required:'+str(e)
 out['requests']=requests
 return out,pins

def cases():
 base=dict(resource_prefix='GER/',model='%sBanner.nif',skin='',Plane=[])
 for planes in [[],[node(decals=[])],[node(decals=[node(textures=['%sBase.png'])])],
  [node(decals=[node('EXPERIENCE',['a','b','a']),node('BANNER_ICON',[]),node('BADGE',[])])],
  [node('EXPERIENCE',decals=[]),node('ENGAGED',decals=[node('NONE',['x'])])],
  [node('BANNER_ICON',decals=[])],[node('NONE',decals=[node('NONE',[])])],
  [node('NONE',decals=[node('NONE',['a','b'])])],[node('UNKNOWN',decals=[])],
  [node('NONE',decals=[node('NONE',[''])])]]:
  yield dict(source={**base,'Plane':planes})
 for stop in [0,1,2]:yield dict(source={**base,'skin':'skin','Plane':[node(decals=[node('EXPERIENCE',['a','b','c'])])]},factory_stop=stop)
 for delta in [0,1,0xffffffff]:
  for mode in [0,1,0xffffffff]:
   for stop in [-1,0,2]:yield dict(source={**base,'Plane':[node(decals=[node('EXPERIENCE',['a','b','a'])])]},delta=delta,mode=mode,request_stop=stop)

def main():
 rows=[];pins={}
 for pc in full.order.PCS:
  for c in cases():
   a,pa=run(h.P,pc,c);b,pb=run(h.V,pc,c);assert a==b;rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for at,raw in pins.items():assert bytes.fromhex(raw)==h.P.range(at,len(bytes.fromhex(raw)))==h.V.range(at,len(bytes.fromhex(raw)))
 doc=dict(schema=1,method=__doc__,rows=rows,instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest());p=HERE/'native-banner-source-request.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
