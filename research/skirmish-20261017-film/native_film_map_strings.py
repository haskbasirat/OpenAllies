"""59FF28..5A003A actual MAPSTRINGS tail with ready wrapper/existing-resource
inputs.4738C0 builder, depot query/add, chunk open, wrapper40/44 and DeleteFileW
are explicit services. Actual late compression/separate reads, CString copies,
real859B30 dispatch/release and wrapper-release call ordering execute. This is
not proof of the earlier unconditional wrapper constructor or child encoding.
Also executes5A01E0's header-then-text dispatcher with required child seams.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Finished(Exception):pass
class Stopped(Exception):pass
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);events=[];seen=[]
 if case['kind']=='dispatch':
  resource=m.alloc(0xb4)
  def child(name):
   if len(events)==case.get('stop',-1):raise Stopped(name)
   events.append(name);assert m.u.reg_read(UC_X86_REG_ECX)==resource;m.ret(0)
  m.seam(0x59ea60,lambda:child('header'));m.seam(0x59fda0,lambda:child('map_strings'))
  stopped=None
  try:m.call(0x5a01e0,resource)
  except Stopped as e:stopped=str(e)
  return dict(events=events,stopped=stopped),m.windows()
 def event(name):
  if len(events)==case.get('stop',-1):raise Stopped(name)
  events.append(name)
 resource=m.alloc(0xb4);wrapper=m.alloc(16);vt=m.alloc(0x50);m.w(wrapper,vt)
 m.w(wrapper+8,m.cstring('Userdata/Films/native.tgi'))
 existing=m.alloc(0x80)if case['existing']else 0
 app=m.alloc(0x98);work=m.alloc(4);m.w(app+0x90,work);m.w(0xb36540,app)
 data=m.alloc(0x600);m.w(0xb36530,data)
 table=m.r(0xb39940);compressed=m.alloc(4);separate=m.alloc(4)
 m.w(table+0x28c,compressed);m.w(table+0x278,separate)
 m.w(compressed,case['compressed']);m.w(separate,case['separate'])
 def built():
  event('build');assert m.u.reg_read(UC_X86_REG_ECX)==data and m.arg(0)==wrapper
  if 'build_compressed'in case:m.w(compressed,case['build_compressed'])
  m.ret(4)
 def has_work():
  event('existing_work');assert m.u.reg_read(UC_X86_REG_ECX)==existing and m.arg(0)==work
  m.ret(4,int(case['has_work']))
 def added():
  event('add_work');assert m.u.reg_read(UC_X86_REG_ECX)==existing and m.arg(0)==work;m.ret(4)
 def chunk():
  event('chunk');assert m.u.reg_read(UC_X86_REG_ECX)==resource+0x3c
  assert m.text(m.arg(0))=='MAPSTRINGS';seen.append(m.arg(1)&255);m.ret(8)
 def serialized():
  event('serialize');assert m.u.reg_read(UC_X86_REG_ECX)==wrapper
  assert m.arg(0)==resource+0x3c and m.arg(1)==0
  if 'serialize_separate'in case:m.w(separate,case['serialize_separate'])
  m.ret(8)
 def deleted():
  event('delete');assert m.text(m.arg(0))=='Userdata/Films/native.tgi'
  if 'delete_separate'in case:m.w(separate,case['delete_separate'])
  m.ret(4,case['delete_result'])
 def saved():
  event('save');assert m.u.reg_read(UC_X86_REG_ECX)==wrapper
  assert m.text(m.r(m.arg(0)))=='Userdata/Films/native.tgi' and m.arg(1)==0;m.ret(8)
 def released():
  event('release');assert m.u.reg_read(UC_X86_REG_ECX)==wrapper and m.arg(0)==1;m.ret(4)
 m.seam(0x4738c0,built);m.seam(0x5aa390,has_work);m.seam(0x5aaae0,added);m.seam(0x859150,chunk)
 for off,fn,stub in [(0x40,serialized,L.STUBS+0xc00),(0x44,saved,L.STUBS+0xc20),(0x10,released,L.STUBS+0xc40)]:
  m.seam(stub,fn);m.w(vt+off,stub)
 #Real859B30 dispatches the resolved DeleteFileW slot and ignores its result.
 stub=L.STUBS+0xc60;m.seam(stub,deleted);m.w(0xb3641c,stub)
 def before(u,at,size,_):
  if at==0x59ff63:event('compression')
  elif at==0x59ffae:event('separate')
  elif at==0x5a003a:raise Finished()
 m.u.hook_add(UC_HOOK_CODE,before)
 sp=L.STACK+L.STACK_SIZE-0x1000;m.w(sp+0x10,m.cstring('Films/native.tgi'))
 m.u.reg_write(UC_X86_REG_ESP,sp);m.u.reg_write(UC_X86_REG_ESI,resource)
 m.u.reg_write(UC_X86_REG_EDI,wrapper);m.u.reg_write(UC_X86_REG_EBX,existing)
 stopped=None
 try:m.u.emu_start(0x59ff28,L.STOP,count=500000)
 except Finished:pass
 except Stopped as e:stopped=str(e)
 else:raise AssertionError('native fragment did not reach bounded exit')
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 return dict(events=events,stopped=stopped,chunk_compression=seen,
             current_separate=m.r(separate)&255),m.windows()
def cases():
 for stop in [None,0,1]:
  c=dict(kind='dispatch')
  if stop is not None:c['stop']=stop
  yield c
 seeds=[]
 for separate in [0,1,0x80]:
  for existing,work in [(False,False),(True,False),(True,True)]:
   seeds.append(dict(kind='tail',separate=separate,compressed=1,existing=existing,has_work=work,delete_result=0))
 seeds += [dict(kind='tail',separate=1,compressed=0,existing=False,has_work=False,delete_result=1,serialize_separate=0),
           dict(kind='tail',separate=0,compressed=0,existing=False,has_work=False,delete_result=1,serialize_separate=0x80),
           dict(kind='tail',separate=1,compressed=0,existing=True,has_work=False,delete_result=0,build_compressed=0x80,delete_separate=0)]
 for c in seeds:
  yield c
  for stop in range(10):yield dict(c,stop=stop)
def main():
 rows=[];blocks={}
 for c in cases():
  for pc in PC:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   rows.append(dict(input=c,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 raw=(json.dumps(dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)]),indent=1)+'\n').encode()
 path=HERE/'native-film-map-strings.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
