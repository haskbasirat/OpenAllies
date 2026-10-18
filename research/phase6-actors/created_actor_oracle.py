"""Whole native Actor ctor and GObject InitCreate, including variant/scale RNG."""
from pathlib import Path
import json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase5-actors'))
from original_creation import Machine,P,V,H,bits
from load_constructors_oracle import WORDS,string

def main():
 tables=json.loads((ROOT/'research/phase5-actors/creation-native-oracle.json').read_text())['tables'];rows=[];visited={}
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  table=next(t['values']for t in tables if t['precision']==name)
  for flags in [0,1<<23,(1<<23)|(1<<15)]:
   for enabled,variant in [(False,-1),(True,-1),(True,2)]:
    for rng in [0,0x12345678,0xffffffff]:
     m=Machine(cw);m.use_table(table);m.u.mem_write(m.obj,b'\xcd'*512)
     inputs={'precision':name,'startup_precision':name,'definition_flags':flags,'rng_state':rng,'world_begin_bits':bits(8.125),'position_bits':[bits(3.25),bits(-1.5),0],'direction_bits':[bits(.6),bits(.8)],'name_utf16':[65,0,90],'variant':variant,'owner_word':0x12340007,'word_30':9,'saved_id':0,'variations_enabled':enabled,'weights_bits':list(map(bits,[1,2,3])),'total_weight_bits':bits(6),'scale_min_bits':bits(1.2),'scale_max_bits':bits(3.7)}
     for off,v in [(4,m.definition),(0x1c,m.string(inputs['name_utf16'])),(0x20,variant&0xffffffff),(0x24,0),(0x28,inputs['owner_word']),(0x30,9)]:m.w(m.record+off,v)
     for i,v in enumerate(inputs['position_bits']):m.w(m.record+8+i*4,v)
     for i,v in enumerate(inputs['direction_bits']):m.w(m.record+0x14+i*4,v)
     m.w(m.definition+0x1c,flags);m.w(m.world+0xe0,inputs['world_begin_bits']);m.w(m.world+0xd4,rng)
     m.u.mem_write(m.definition+0x108,bytes([int(enabled)]));m.w(m.definition+0x10c,0x225a000);m.w(m.definition+0x128,inputs['total_weight_bits']);m.w(m.definition+0x28,inputs['scale_min_bits']);m.w(m.definition+0x2c,inputs['scale_max_bits'])
     for i,weight in enumerate(inputs['weights_bits']):p=0x225b000+i*0x40;m.w(0x225a000+i*4,p);m.w(p+0x2c,weight)
     m.invoke(0x4d6bb0,m.obj,[m.record])
     ctor={'flags':m.r(m.obj+8),'name_utf16':string(m,m.r(m.obj+24)),'created_at_bits':m.r(m.obj+28),'position_bits':[m.r(m.obj+32+4*i)for i in range(3)],'facing_bits':[m.r(m.obj+44+4*i)for i in range(3)],'variant':m.r(m.obj+0x40),'actor_words':{str(i*4):m.r(m.obj+i*4)for i in WORDS},'components':[m.r(m.obj+0x60+4*i)for i in range(31)],'state':[m.r(m.obj+0x100),m.r(m.obj+0x104)]}
     owner=m.r(m.record+0x28);after=m.r(m.world+0xd4)
     initialized=m.invoke(0x4e9fc0,m.obj,[m.record])&255
     rows.append({'input':inputs,'constructor':ctor,'record_owner_after':owner,'rng_after_constructor':after,'initialized':bool(initialized),'id':m.r(m.obj+20),'scale_bits':m.r(m.obj+0x3c),'rng_after_initialize':m.r(m.world+0xd4)});visited.update(m.visited)
 spans=[]
 for a,n in sorted(visited.items()):
  if not P.base<=a<P.base+P.image_size:continue
  if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
  else:spans.append([a,a+n])
 pins=[]
 for a,z in spans:
  raw=P.range(a,z-a);assert raw==V.range(a,z-a);pins.append({'start':hex(a),'end':hex(z),'sha256':H(raw)})
 p=HERE/'created-actors-native.json';p.write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'native_windows':pins,'seams':['WholeActor4D6BB0->GObject4EA5E0->GThing4EF2A0 andGObjectInitCreate4E9FC0 execute unmodified.','Stringpool successfulallocation/retention fixtureleaves; original weightedselector49B900 andscale49B440 execute.','Record+52 nonblocking callback false; spatial registration branch not substituted.','No ActorInitCreate4D6F10 owner/component/ability/worldpublication work claimed.']},indent=2)+'\n');print(len(rows),H(p.read_bytes()))
if __name__=='__main__':main()
