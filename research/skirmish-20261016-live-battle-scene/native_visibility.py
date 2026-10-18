"""Unchanged native visibility getters with explicit view-global inputs.
4EEF70/4EA1D0/4D3B80/4D3C50/54EB40/54EB70/4364C0 execute, including
actual class vtables and live Organization/Element pointers. Only OS thread
identity is seamed. No full renderer, player bootstrap, fog-variable producer,
positive Flare5733A0 or remembered-model-selection proof is claimed.
"""
from pathlib import Path
import copy, hashlib, json, sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P
HASH=lambda b:hashlib.sha256(b).hexdigest()
VT={2:0xa2a2a0,3:0xa273f0,4:0xa299d8,6:0xa28a00,7:0xa28b88,9:0xa28e90}

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 app=m.alloc(0x100);thread=m.alloc(0x80);world=m.alloc(0x300);variables=m.alloc(0x200)
 fog=m.alloc(4);editor=m.alloc(4);m.w32(fog,c['fog']);m.w32(editor,c['editor_option'])
 m.w32(0xb36550,app);m.w32(app+0xe4,c['application_state']);m.w32(0xb364d4,thread);m.w32(thread+0x48,77)
 m.w32(0xb36534,world);m.w8(world+0x275,c['editor_world']);m.w32(0xb39ab0,variables)
 m.w32(variables+0x108,fog);m.w32(variables+0x1bc,editor)
 kingdom=m.alloc(0x300);m.w32(kingdom+0x190,c['nonmajor']);m.w32(kingdom+0x228,c['shared_mask'])
 actors={int(k):m.alloc(0x200)for k in c['actors']}
 for key,a in actors.items():
  d=c['actors'][str(key)];m.w32(a,VT[d['class']]);m.w32(a+8,d['flags']);m.w32(a+0x54,d['detection'])
  if d['members']is not None:
   org=m.alloc(0x100);array=m.alloc(max(4,len(d['members'])*4))
   m.w32(org+0x28,array,len(d['members']));m.w32(a+0x7c,org)
   for i,k in enumerate(d['members']):m.w32(array+4*i,actors.get(k,0))
  if d['element']is not None:
   element=m.alloc(0x30);m.w32(element+0x10,actors.get(d['element'],0));m.w32(a+0x80,element)
 reads=[]
 def thread_id(_):reads.append('thread');m.ret(value=77 if c['simulation_thread']else 88)
 stub=m.stub_next;m.stub_next+=16;m.u.mem_write(stub,b'\xc3');m.seams[stub]=thread_id;m.w32(0x9f01e4,stub)
 def observe(u,access,address,size,value,_):
  if address==fog:reads.append('fog')
  if address==app+0xe4:reads.append('application')
  if address==editor:reads.append('editor_option')
  if address==world+0x275:reads.append('editor_world')
 m.u.hook_add(UC_HOOK_MEM_READ,observe)
 target=kingdom if c['kingdom']else 0
 entry={'visible':0x4eef70,'visible_to':h.P.u32(VT[c['actors']['1']['class']]+0x70),
        'explored_by':h.P.u32(VT[c['actors']['1']['class']]+0x74)}[c['method']]
 result=m.call(entry,actors[1],[target,int(c['deep'])])&255
 assert m.final_esp==h.STACK+12;assert m.fpu_top()==0
 return dict(result=result!=0,view_reads=reads)

def actor(word=0,cls=3,members=None,element=None,flags=0):
 return dict(class_=cls,detection=word,members=members,element=element,flags=flags)
def normalize(a):
 a=copy.deepcopy(a);a['class']=a.pop('class_');return a
def cases():
 base=dict(method='visible_to',fog=1,kingdom=True,nonmajor=0,shared_mask=0x00010001,
  application_state=3,simulation_thread=False,editor_option=False,editor_world=False,deep=False,
  actors={'1':normalize(actor())})
 out=[]
 for cls in VT:
  for method in ('visible_to','explored_by'):
   for fog,kingdom,nonmajor,mask,word in ((0,True,0,0,0),(1,False,0,0,0),(1,True,1,0,0),
     (1,True,0,0x10001,0),(1,True,0,0x10001,1),(1,True,0,0x10001,0x10000),
     (1,True,0,0xffffffff,0xffffffff),(1,True,0,0x80008000,0x80008000)):
    c=copy.deepcopy(base);c.update(method=method,fog=fog,kingdom=kingdom,nonmajor=nonmajor,shared_mask=mask)
    c['actors']['1']=normalize(actor(word,cls));out.append(c)
 for dead in (False,True):
  for simulation in (False,True):
   for state,option,override in ((3,False,False),(0,False,False),(0,True,False),(0,False,True)):
    c=copy.deepcopy(base);c.update(method='visible',simulation_thread=simulation,application_state=state,
      editor_option=option,editor_world=override)
    c['actors']['1']['flags']=128 if dead else 0;out.append(c)
 for method in ('visible_to','explored_by'):
  for deep in (False,True):
   for members,element,first,second in (([],None,0,0),([0,2,3],None,0,0x10001),
    ([2,2,0],3,0,0x10001),(None,2,0x10001,0),(None,2,0,0),([2,3],None,0x10001,0)):
    c=copy.deepcopy(base);c.update(method=method,deep=deep)
    c['actors']={'1':normalize(actor(members=members,element=element)),
      '2':normalize(actor(first)),'3':normalize(actor(second))};out.append(c)
 return out

def main():
 rows=[]
 for pc in h.PRECISION:
  for c in cases():
   a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b
   rows.append(dict(input=dict(c,precision=pc),output=a))
 pins=[]
 for cls,vt in VT.items():
  for slot in (0x5c,0x70,0x74):
   b=ORIGINAL.range(vt+slot,4);assert b==h.V.range(vt+slot,4)
   pins.append(dict(address=hex(vt+slot),bytes=b.hex()))
 result=dict(schema=1,scope=__doc__,original_sha256=HASH(ORIGINAL.b),v14_sha256=HASH(h.V.b),
  cases=rows,native_executions=len(rows)*2,data_pins=pins,
  instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-visibility.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(p.read_text())==result
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=HASH(p.read_bytes()))))
if __name__=='__main__':main()
