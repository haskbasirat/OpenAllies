"""Actual4EC4A0 Supply memory stats4CECB0/4CECE0->58AA10.
Supply is Actor+8C slot11; deliberately poisoned Denizen+88 is never read.
Relay false and absent Supply skip both stockpile/current and source maximum.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-store-memory-refresh'))
import native_memory_refresh as b
h=b.h;H=lambda v:hashlib.sha256(v).hexdigest()
class Machine(b.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc,image,c)
  self.w(self.actor+0x88,0x12345000)
  self.wb(self.definition+0x424,int(c['is_relay']))
  self.w(self.definition+0x428,c['stockpile_max'])
  if c['supply_present']:
   supply=self.alloc(0x20,0xa5);self.w(self.actor+0x8c,supply);self.w(supply+4,self.actor);self.w(supply+0x18,c['stockpile'])
def main():
 rows=[]
 for present in [False,True]:
  for relay in [False,True]:
   for maximum,current in [(1000.0,125.0),(17.25,1.5)]:
    for stop in ['none','stat4','stat5']:
     c={'class':3,'time':0x40800000,'flags':0x40a12400,'position':[0x3fc00000,0xc0200000],'variant':17,'owner':781,'display':'Supply','key':'','text':'','kingdom':'','area':'','camera':'','stop':stop,'data_variations':True,'supply_present':present,'is_relay':relay,'stockpile_max':h.f32(maximum)if relay else 0x7fc12345,'stockpile':h.f32(current)if relay else 0x7fc54321,'denizen_pointer':'deliberately_unread_poison'}
     for pc in h.CW:
      out=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert out[0]==out[1]
      if stop=='none':assert out[0]['stats']==[0,0,c['stockpile']if present and relay else 0,c['stockpile_max']if present and relay else 0,0]
      rows.append({'input':c,'pc':pc,'native':out[0]})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':v,'kind':'executed_or_explicit_required_stat_entry'}for a,v in sorted(h.VISITED.items())]}
 p=HERE/'native-memory-supply-source.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
