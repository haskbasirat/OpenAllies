"""Whole515620 slot4 getter ->514DA0 ->514E40 in both unchanged images.

Laboratory actor/zone fields are explicit inputs, not current mission values.
Owner108, enemy140, and positive component bodies are named transport seams;
the actual guard, redirect selection, null checks and child order execute.
Component transports may replace B0 to prove its fresh read after57AB70.
No positive component implementation, relation producer or startup claim.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
P,V=h.P,h.V
def cases():
 base=dict(flags=0x4000,leaf=16,count=9,zone=True,zone_flags=16,enemy=1,
           element=False,redirect=False,enabled=1,org=False,defender=False,guard=False,
           mutation='none',cut='none')
 out=[]
 def add(name,**kw):out.append(dict(base,name=name,**kw))
 add('null-components');add('no-membership',leaf=0)
 add('not-actor',flags=0);add('dead-actor',flags=0x4001)
 add('null-zone-ineligible',zone=False,flags=0);add('null-zone-dead',zone=False,flags=0x4001)
 add('null-zone',zone=False);add('short-vector',count=4)
 add('not-enemy',enemy=0,org=True,defender=True)
 add('org',org=True);add('defender',defender=True);add('both',org=True,defender=True)
 add('both-flags-guard-exclusive',zone_flags=0x30,guard=True,org=True,defender=True)
 add('zone-other-type',zone_flags=8,org=True,defender=True)
 add('redirect-null',element=True,org=True)
 add('redirect-disabled',element=True,redirect=True,enabled=0,org=True)
 add('redirect',element=True,redirect=True,org=True,defender=True)
 add('redirect-raw128',element=True,redirect=True,enabled=128,org=True)
 add('org-removes-b0',org=True,defender=True,mutation='clear')
 add('org-installs-b0',org=True,mutation='install')
 add('org-replaces-b0',org=True,defender=True,mutation='replace')
 add('required-org',org=True,defender=True,cut='org')
 add('required-defender',org=True,defender=True,cut='defender')
 add('required-guard',zone_flags=0x20,guard=True,cut='guard')
 return out
class Cut(Exception):pass
def run(c,pc,image):
 h.P=image;m=h.Machine(pc)
 leaf,zoneactor,actor,slots,zone,vt,zvt,element,redirect=[m.alloc(n)for n in(16,0x200,0x200,36,24,0x180,0x180,32,0x200)]
 kingdom=m.alloc(4);components={'org':m.alloc(32),'defender':m.alloc(32),'guard':m.alloc(32),'replacement':m.alloc(32)}
 m.w32(leaf+8,zoneactor,c['leaf']);m.w32(zoneactor,zvt);m.w32(actor,vt)
 m.w32(zoneactor+0x148,slots,c['count']);m.w32(slots+16,zone if c['zone']else 0)
 m.w32(zone,0xa3229c,zoneactor);m.w32(zone+20,c['zone_flags'])
 m.w32(actor+8,c['flags']);m.w32(actor+0x80,element if c['element']else 0)
 m.w32(element+16,redirect if c['redirect']else 0);m.w8(element+28,c['enabled'])
 for offset,name in[(0x7c,'org'),(0xb0,'defender'),(0x88,'guard')]:m.w32(zoneactor+offset,components[name]if c[name]else 0)
 events=[]
 def owner(mm):events.append(['owner']);mm.ret(value=kingdom)
 def enemy(mm):assert mm.arg()==kingdom;events.append(['enemy']);mm.ret(4,value=c['enemy'])
 m.w32(zvt+0x108,h.STUBS);m.seams[h.STUBS]=owner
 m.w32(vt+0x140,h.STUBS+16);m.seams[h.STUBS+16]=enemy
 def child(name):
  def callback(mm):
   target=mm.arg();assert target in(actor,redirect)
   comp=next(k for k,v in components.items()if v==mm.ecx())
   events.append([name,comp,'redirect'if target==redirect else'actor'])
   if c['cut']==name:raise Cut()
   if name=='org':
    if c['mutation']=='clear':m.w32(zoneactor+0xb0,0)
    elif c['mutation']in('install','replace'):m.w32(zoneactor+0xb0,components['replacement'])
   mm.ret(4)
  return callback
 boundaries={0x57ab70:'org',0x567840:'defender',0x568d80:'guard'}
 for address,name in boundaries.items():m.seams[address]=child(name)
 terminal='returned'
 try:
  m.call(0x515620,leaf,[actor]);assert m.final_esp==h.STACK+8 and m.fpu_top()==0
 except Cut:terminal='required-'+c['cut']
 except h.NullDereference:terminal='null-zone'
 return dict(events=events,terminal=terminal)
def main():
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.PRECISION,cases()):
  a=run(c,pc,P);b=run(c,pc,V);assert a==b,(c,pc,a,b)
  rows.append(dict(precision=pc,input=c,native=a))
 h.P=P;seams={0x57ab70,0x567840,0x568d80}
 #Null-fault instructions are reached but do not execute successfully.
 fault={0x514db8}
 doc=dict(schema=1,scope=__doc__,rows=rows,
  executed_pins={hex(a):b for a,b in sorted(h.VISITED.items())if a not in seams|fault},
  intercepted_pins={hex(a):b for a,b in sorted(h.VISITED.items())if a in seams},
  null_fault_pins={hex(a):b for a,b in sorted(h.VISITED.items())if a in fault},
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),harness_sha256=hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest())
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-control-entry.json'
 if '--write'in sys.argv:
  with path.open('xb')as f:f.write(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
