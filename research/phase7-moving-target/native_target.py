"""Original533C30 with complete ordinary Element5317A0 and46E4F0/46E150.
Element Mind+A4 executes actual52D780 (native+184==2). Mode2 runs only to the
unsupported formation-placement branch531995; it is not counted as a complete
update. Radius/Performer maximum/speed
virtuals and actor query construction/result are explicit fixture services.
Actual target generation checks, soft getter, projection, distance stores,
state activation checks, ignored-target replacement and path reset execute.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-element-move'))
import native_element_move as base
from unicorn.x86_const import *
DEFS=0x4090000;SPEED=base.HOOK+0xa00
f64=lambda f:struct.unpack('<Q',struct.pack('<d',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(base.Machine):
 def __init__(self,c):
  super().__init__(c)
  self.w(base.VT+0x10,0x4ea020);self.w(base.VT+0x18,SPEED);self.w(base.VT+0x400+0xa4,0x52d780);self.w(base.mind(1)+0x184,c['formation_mode'])
  for i in range(1,4):
   d=DEFS+i*0x1000;self.w(base.actor(i)+4,d);self.put(d+0x50,*c['shape']);self.w(base.actor(i)+0x7c,0)
  if 'target_facing'in c:self.put(base.actor(3)+0x2c,*c['target_facing'])
  for offset in [0xf8,0x164]:self.w(base.mind(1)+offset+4,base.mind(1))
  if c['active_offset']is not None:self.w(base.mind(1)+0x200,base.mind(1)+c['active_offset'])
 def mutation(self,name):
  super().mutation(name);m=self.c['mutations'].get(name,{})
  if 'target_id'in m:self.w(self.STATE+0x10,m['target_id'])
  if 'target_flags'in m:self.w(base.actor(3)+8,m['target_flags'])
  if 'previous'in m:self.put(self.STATE+0x34,*m['previous'])
 def hook(self,u,pc,size,arg):
  cx=u.reg_read(UC_X86_REG_ECX);sp=u.reg_read(UC_X86_REG_ESP)
  if pc==0x533c30:
   self.visited[pc]=base.original.range(pc,size).hex();return
  if pc==0x4ea020:
   self.log('soft_shape',actor=(cx-base.ACT)//0x400)
  elif pc==0x554350:
   self.log('actor_query',actor=(self.r(sp+4)-base.ACT)//0x400,strict=bool(self.r(sp+8)))
   self.u.mem_write(cx,bytes(0x58));self.ret(cx,8);return
  elif pc==0x4ce6e0:self.log('replacement_query_key',actor=(cx-base.ACT)//0x400)
  elif pc==0x552250:
   self.log('set_query_position',point=[self.r(sp+4),self.r(sp+8)])
  elif pc==0x553ba0:
   out=self.r(sp+4);query=self.r(sp+8);ignored=self.r(query+0x20)
   self.log('first_hit',point=[self.r(query+0x4c),self.r(query+0x50)],ignore_moving=bool(self.u.mem_read(query+0x31,1)[0]),ignored=None if not ignored else(ignored-base.ACT)//0x400)
   self.put(out,self.c['hit'],0,0,0);self.ret(out,8);return
  elif pc==0x57ec80:self.log('maximum_range',performer=cx);self.fp(self.c['maximum64'],True);return
  elif pc==SPEED:self.log('speed',actor=(cx-base.ACT)//0x400);self.fp(self.c['speed64'],True);return
  super().hook(u,pc,size,arg)
 def run_target(self):
  self.u.emu_start(0x533c30,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  out=self.output();raw=self.u.reg_read(UC_X86_REG_EAX);out['result']=None if not raw else(raw-base.ACT)//0x400;return out
 def run_prefix(self):
  self.u.emu_start(0x533c30,0x531995,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==0x531995
  return self.output()
def cases():
 rows=[]
 for shape in [[0,0,0,0,0,0],[1,base.F(2),0,0,0,0],[2,base.F(2**.5),base.F(-1),base.F(-1),base.F(1),base.F(1)],[3,base.F(5**.5),base.F(-2),base.F(-1),base.F(2),base.F(1)]]:
  for active in [None,0xf8,0x164]:
   for hit in [0,2]:
    for point in [[12,5],[5.5,5],[5,5],[5.5,5.5]]:
     c=base.base();c.update(shape=shape,active_offset=active,maximum64=f64(4.1234567),speed64=f64(.717),hit=hit);c['actors']['1']['component74']=74;c['actors']['2']['component74']=75;c['callbacks']['radius64']=f64(1.0000000001);c['actors']['3']['position']=list(map(base.F,point));p=c['payload'];p['plan']['movement'].update(target_id=0x10003,mode=2);p['plan']['movement']['path'].update(nodes=[{'words':[base.F(6),base.F(7),0,0,1,0,0,0]}],kind1_count=1,word18=8);p['plan']['previous_destination_bits']=list(map(base.F,[5,5]));rows.append(c)
 for mutate in [{'radius':{'owner':2}},{'speed':{'target_flags':1<<29}},{'speed':{'previous':list(map(base.F,[12,5]))}},{'maximum_range':{'target_id':0x10002}}]:
  c=copy.deepcopy(rows[8]);c['active_offset']=0xf8;c['mutations']=mutate;rows.append(c)
 c=copy.deepcopy(rows[0]);c['payload']['plan']['movement']['target_id']=0x20003;rows.append(c)
 for mode in [1,3,0xffffffff]:
  c=copy.deepcopy(rows[0]);c['formation_mode']=mode;rows.append(c)
 for point in [[5.25,5.5],[12,5]]:
  for facing in [[1,0],[.6,-.8],[-.8,-.6]]:
   c=copy.deepcopy(rows[50]);c['actors']['3']['position']=list(map(base.F,point));c['target_facing']=list(map(base.F,facing));rows.append(c)
 for pc in ['Single24','Double53','Extended64']:
  for i,c in enumerate(rows):yield {**copy.deepcopy(c),'formation_mode':c.get('formation_mode',0),'name':f'{pc}_{i}','precision':pc}
def main():
 rows=[];prefixes=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run_target()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':out});visited.update(m.visited)
 selected=list(cases())
 for c in [selected[i] for i in [0,48,50,54]]:
  c=copy.deepcopy(c);c['formation_mode']=2;c['name']='unsupported_formation_'+c['name'];m=Machine(c);prefixes.append({'input':c,'output':m.run_prefix()});visited.update(m.visited)
 for a,h in visited.items():b=bytes.fromhex(h);assert b==base.original.range(a,len(b))==base.modified.range(a,len(b)),hex(a)
 ranges={}
 for label,a,end in [('update',0x533c30,0x533d8f),('projection',0x5317a0,0x531a52),('placed_projection',0x46e4f0,0x46e53f),('element_formation_dispatch',0xa35810+0xa4,0xa35810+0xa8),('element_formation_mode',0x52d780,0x52d78f),('threshold',0xa3547c,0xa35480)]:
  b=base.original.range(a,end-a);assert b==base.modified.range(a,end-a);ranges[label]={'va':hex(a),'length':end-a,'sha256':sha(b)}
 doc={'schema':1,'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(base.original.b),'accepted_sha256':sha(base.modified.b),'ranges':ranges,'cases':rows,'unsupported_prefix_cases':prefixes,'visited_windows':[{'va':hex(a),'bytes':h}for a,h in sorted(visited.items())]}
 (HERE/'native-target-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
