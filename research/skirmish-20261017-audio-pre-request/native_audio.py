"""AudioData46B400 constructor/46B1D0 request and Sound59B400 constructor.
Real base5AA890/CommonTask,59B1F0/59B4B0 type/store paths,9D0900 class table.
Heap/CString/atomic are inherited host transports. Cached/source Find and
Sound5AA3F0 request are explicit leaf transports; no audio decode/playback.
"""
from pathlib import Path
import sys,json,hashlib,gc,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_ECX,UC_X86_REG_FPCW,UC_X86_REG_EAX,UC_X86_REG_ESI,UC_X86_REG_EDI
class Required(Exception):pass
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def inc(m,u):
  p=m.r(u.reg_read(UC_X86_REG_ESP)+4);v=(m.r(p)+1)&0xffffffff;m.w(p,v);m.ret(v,pop=4)
 m.stubs[0x301f750]=inc;m.w(0x9f01e8,0x301f750)
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 m.thiscall(0x4013e0,0xb43b40,[])
 sounds=[];ctors=[]
 for name in ['Sounds/General/CallOuts/CityBuildingSelect.wav','Music/test.MP3','test.wav']:
  slot=m.alloc(4);m.w(slot,m.cstring(name));resource=m.cdecl(0x59b400,[slot]);sounds.append(resource)
  task=m.r(resource+0x20)
  ctors.append(dict(name=name,vtable=hex(m.r(resource)),name_retained=m.wstr(m.r(resource+0x38)),base={hex(i):m.r(resource+i)for i in [4,8,12,16,20,24,28,40,44,48,52]},derived={'0x3c':m.r(resource+0x3c),**{hex(i):m.u.mem_read(resource+i,1)[0]for i in [0x40,0x41,0x42]}},task={hex(i):m.r(task+i)for i in [4,8,12,16,20,24]},same_task_resource=m.r(task+0x1c)==resource))
 m.cdecl(0x9d0900,[]);extensions=[]
 for i in range(21):
  text=m.raw_wstr(m.r(0xb282f8+i*8));descriptor=m.r(0xb282fc+i*8)
  if text in ['WAV','MP3']:extensions.append(dict(extension=text,descriptor=hex(descriptor)))
 #Exact descriptor-name input only reaches a diagnostic, never type success.
 m.w(0xb3d770,m.cstring('KKC_AudioResource'))
 current={};lookups=[]
 def find(m,u):current['trace'].append('cached');m.ret(current['resource']if current['cached']else 0,pop=4)
 def create(m,u):current['trace'].append('source');m.ret(current['resource'],pop=8)
 def diagnostic(m,u):current['trace'].append('diagnostic');raise Required()
 m.stubs[0x5ad430]=find;m.stubs[0x5aea90]=create;m.stubs[0x425210]=diagnostic;m.stubs[0x4252a0]=diagnostic
 for cached,present,streaming in itertools.product([False,True],repeat=3):
  current={'trace':[],'cached':cached,'resource':sounds[0]if present else 0}
  m.u.mem_write(sounds[0]+0x41,b'\0');slot=m.alloc(4);m.w(slot,m.cstring('city.wav'));out=None;stop=None
  try:out=m.cdecl(0x59b4b0,[slot,int(streaming)])
  except Required:stop='diagnostic_required'
  lookups.append(dict(cached=cached,present=present,streaming=streaming,same_resource=out==sounds[0]if out else None,stop=stop,trace=current['trace'],byte41=m.u.mem_read(sounds[0]+0x41,1)[0]))
 #Actual Audio constructor has no shared Data20 request counter.
 slot=m.alloc(4);m.w(slot,m.cstring('city_building_select'));audio=m.alloc(0x88);m.u.mem_write(audio,b'\xa5'*0x88)
 m.thiscall(0x46b400,audio,[slot]);audio_ctor=dict(vtable=hex(m.r(audio)),words={hex(i):m.r(audio+i)for i in [12,24,28,36,40,44,48,56,60,68,72,80,84,88,92,96,100,104,108,112,116,120,124,132]},bytes={hex(i):m.u.mem_read(audio+i,1)[0]for i in [4,20,32,33,34]},ids=m.wstr(m.r(audio+8)),empty10=m.wstr(m.r(audio+16)),empty80=m.wstr(m.r(audio+128)))
 requests=[];trace=[];stop_at=None
 def request(m,u):
  resource=u.reg_read(UC_X86_REG_ECX);sp=u.reg_read(UC_X86_REG_ESP)
  trace.append(dict(sound=sounds.index(resource),delta=m.r(sp+4),mode=m.r(sp+8)))
  if stop_at==len(trace):raise Required()
  m.ret(pop=8)
 m.stubs[0x5aa3f0]=request
 shapes=[([],[],[]),([0],[],[]),([],[1],[]),([],[],[2]),([0,0],[1,0],[2,1,2])]
 for shape,delta,mode in itertools.product(shapes,[0,1,0xffffffff,0x80000000],[0,1,256,255]):
  for offset,ids in zip([0x34,0x40,0x4c],shape):
   array=m.alloc(max(1,len(ids))*4)
   for i,id in enumerate(ids):m.w(array+i*4,sounds[id])
   m.w(audio+offset,array);m.w(audio+offset+4,len(ids));m.w(audio+offset+8,len(ids))
  for stop_at in range(0,sum(map(len,shape))+1):
   trace=[];stop=None
   try:m.thiscall(0x46b1d0,audio,[delta,mode])
   except Required:stop='request_required'
   requests.append(dict(lists=[list(v)for v in shape],delta=delta,mode=mode,stop_at=stop_at,trace=trace,stop=stop))
 #Selected actual source suffix starts at the ignore_rate result already in
 #AL and stops before ldf_length. It runs the real tokenizer, path operation,
 #typed Sound lookup, appends and rate-byte stores. GetString and named Find
 #are declared source transports, not a completed whole Audio LoadData.
 source_cases=[];source_now={};context=m.alloc(16);m.w(0xb36530,context)
 def source_string(m,u):
  key=m.raw_wstr(m.r(u.reg_read(UC_X86_REG_ESP)+4));source_now['reads'].append(key)
  value=m.alloc(4);m.w(value,m.cstring(source_now['fields'][key]));m.ret(value,pop=4)
 def source_find(m,u):
  slot=m.r(u.reg_read(UC_X86_REG_ESP)+4);name=m.wstr(m.r(slot));source_now['finds'].append(name)
  if source_now['stop_at']==len(source_now['finds']):source_now['stopped']='source_required';raise Required()
  resource=sounds[source_now['mapping'].setdefault(name,len(source_now['mapping'])%3)];m.ret(resource,pop=4)
 m.stubs[0x5b54d0]=source_string;m.stubs[0x5ad430]=source_find
 def stop_suffix(u,at,size,_):
  if at==0x46c67c:source_now['stopped']='before_ldf_length';raise Required()
 hook=m.u.hook_add(UC_HOOK_CODE,stop_suffix)
 samples=[['','',''],['A.wav,B.wav','B.wav','A.wav'],['  A.wav,, B.wav , ','/C.wav','A.wav'],['/Sounds/General/x.wav','','']]
 for fields,category,ignore,folder in itertools.product(samples,[0,1,2,0xffffffff],[0,1,255],['','Data/Audio']):
  for stop in range(sum(len([s for s in value.replace(',',' ').split(' ')if s])for value in fields)+1):
   for resource in sounds:m.u.mem_write(resource+0x40,b'\0\0\0')
   for offset in [0x34,0x40,0x4c]:m.w(audio+offset+4,0)
   m.w(audio+0x18,category);m.w(context+4,m.cstring(folder))
   source_now={'fields':dict(zip(['leadin','files','fallout'],fields)),'reads':[],'finds':[],'mapping':{},'stop_at':stop,'stopped':None}
   sp=m.stack;m.w(sp,0x3000000);m.u.reg_write(UC_X86_REG_ESP,sp);m.u.reg_write(UC_X86_REG_EAX,ignore);m.u.reg_write(UC_X86_REG_ESI,audio);m.u.reg_write(UC_X86_REG_EDI,context)
   try:m.u.emu_start(0x46c5c4,0,count=100000)
   except Required:pass
   m.check_null()
   assert source_now['stopped']==('source_required'if stop else'before_ldf_length'),source_now
   source_cases.append(dict(fields=fields,category=category,ignore=ignore,folder=folder,stop_at=stop,reads=source_now['reads'],finds=source_now['finds'],lists=[[sounds.index(m.r(m.r(audio+o)+i*4))for i in range(m.r(audio+o+4))]for o in [0x34,0x40,0x4c]],sound_bytes=[[m.u.mem_read(r+i,1)[0]for i in [0x40,0x41,0x42]]for r in sounds]))
 m.u.hook_del(hook)
 return dict(sound_constructors=ctors,extensions=extensions,lookups=lookups,audio_constructor=audio_ctor,requests=requests,source_projection=source_cases),pins,seams
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 for a,b in pins.items():assert bytes.fromhex(b)==h.P.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
 metadata=h.P.range(0x9d02f0,0x2a);assert metadata==h.V.range(0x9d02f0,len(metadata))
 doc=dict(scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted_not_executed=[hex(a)for a in sorted(seams)],metadata_only={'9D02F0':metadata.hex()},producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-audio.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(machines=6,sound_constructor_calls=18,audio_constructor_calls=6,lookup_calls=sum(len(r['native']['lookups'])for r in rows)*2,request_calls=sum(len(r['native']['requests'])for r in rows)*2,source_suffix_calls=sum(len(r['native']['source_projection'])for r in rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
