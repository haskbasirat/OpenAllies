"""Unchanged4F8AF0 world registration, including exact partial mutation stops.

Successful heap allocation/free is an explicit service seam. Invalid signed
indexes and unavailable owned color storage stop before the native unsafe read;
these are bounded Rust ownership refusals, not claims of native valid inputs.
"""
from pathlib import Path
import sys,json,hashlib,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL=nm.original

def inputs():
 base=dict(name='base',count=0,all=[None,None],majors=[None,None],colors=[0,0],
  all_index=0,major_index=0,minor=0,color=None,owner=781)
 out=[]
 def add(name,**kw):
  c=copy.deepcopy(base);c.update(kw);c['name']=name;out.append(c)
 add('base');add('minor',minor=1,major_index=0xffffffff)
 add('all_wrap',count=0xffffffff,colors=[0xffffffff],color=0)
 add('color_second',color=1)
 add('all_append_exact_end',all=[],majors=[])
 add('append_second',all=[12],majors=[12],all_index=1,major_index=1)
 add('all_occupied_other',all=[12,None])
 add('all_occupied_same',all=[781,None])
 add('major_occupied_other',majors=[12,None])
 add('major_occupied_same',majors=[781,None])
 add('all_negative',all_index=0xffffffff)
 add('major_negative',major_index=0xffffffff)
 add('all_sparse',all_index=4)
 add('major_sparse',major_index=4)
 add('color_storage_absent',colors=None,color=0)
 add('color_outside',color=2)
 return out

def run(c,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified
 m=nm.Machine(pc);world=m.alloc(0x200);kingdom=m.alloc(0x400)
 ids={c['owner']:kingdom};reverse={kingdom:c['owner']}
 for value in c['all']+c['majors']:
  if value is not None and value not in ids:
   p=m.alloc(0x400);ids[value]=p;reverse[p]=value
 m.w(world+0x144,c['count'])
 for offset,values in ((0x148,c['all']),(0x154,c['majors'])):
  m.w(world+offset,m.array([ids[v] if v is not None else 0 for v in values]))
  m.w(world+offset+4,len(values));m.w(world+offset+8,len(values))
 m.w(kingdom+0x188,c['all_index']);m.w(kingdom+0x18c,c['major_index']);m.w(kingdom+0x190,c['minor'])
 colors=c['colors'];cp=m.array(colors) if colors is not None else 0;m.w(world+0x160,cp)
 if c['color'] is not None:
  color=m.alloc(16);m.w(color+12,c['color']);m.w(kingdom+0x1e8,color)
 stop=None
 def halt(label):
  nonlocal stop
  stop=label;m.u.emu_stop()
 def index_read(which):
  idx=c['all_index'] if which=='all' else c['major_index']
  if idx&0x80000000:halt(which+'_negative')
 m.observe(0x4f8b15,lambda mm:index_read('all'))
 m.observe(0x4f8b77,lambda mm:index_read('major'))
 def growth(mm):
  offset=mm.ecx()-world
  which='all' if offset==0x148 else 'major'
  idx=c['all_index'] if which=='all' else c['major_index']
  if idx>mm.r(mm.ecx()+4):halt(which+'_sparse')
  # Observer otherwise executes actual50B530 and4849F0.
 m.observe(0x50b530,growth)
 m.seam(0x6f7810,lambda mm:mm.ret(mm.alloc(mm.arg(1))))
 m.seam(0x6f7430,lambda mm:mm.ret())
 m.seam(0x4f0110,lambda mm:halt('all_occupied' if mm.arg(3)==0x60c else 'major_occupied'))
 def color_read(mm):
  if colors is None:halt('color_storage_absent')
  elif c['color']>=len(colors):halt('color_outside')
 m.observe(0x4f8bd1,color_read)
 try:m.call(0x4f8af0,world,[kingdom])
 except AssertionError:
  if stop is None:raise
 assert not m.null_accesses
 def vector(offset):
  values=[m.r(m.r(world+offset)+4*i) for i in range(m.r(world+offset+4))]
  return [reverse[v] if v else None for v in values]
 output=dict(count=m.r(world+0x144),all=vector(0x148),majors=vector(0x154),
  colors=[m.r(cp+4*i) for i in range(len(colors))] if colors is not None else None,
  stop=stop)
 return output,m.visited

def main():
 rows=[];visited={}
 for c in inputs():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   for pins in (pa,pb):
    for va,raw in pins.items():
     assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
     visited[va]=raw
   rows.append(dict(input=c,precision=pc,output=a))
 fixture=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_runs=len(rows)*2,
  cases=rows,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(visited.items())])
 path=HERE/'native-registration.json'
 if '--write'in sys.argv:path.write_text(json.dumps(fixture,indent=2)+'\n')
 else:assert json.loads(path.read_text())==fixture
 print(json.dumps(dict(cases=len(rows),runs=len(rows)*2,pins=len(visited),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
