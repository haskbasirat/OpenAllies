"""Actual4EC4A0 memory Body getters, especially4CEC90->529C30->45D730(NULL).
4CEC90 pushesNULL, so virtual114 reads ActorData+284, not live maximum or
Properties modifier. Explicit current/source/maximum/poisoned Properties fields
are guard probes; resource/source compilation is separately bound by Rust.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-store-memory-refresh'))
import native_memory_refresh as b
h=b.h;H=lambda v:hashlib.sha256(v).hexdigest()
class Machine(b.Machine):
 def __init__(self,pc,image,c):
  super().__init__(pc,image,c)
  self.w(self.actor+0x78,0x12345000) #would fault if the null-modifier native branch were changed
  self.w(self.definition+0x284,c['source_health'])
  if c['body_present']:
   body=self.alloc(0x50,0xa5);self.w(self.actor+0x60,body);self.w(body+4,self.actor)
   self.w(body+0x10,c['current_health']);self.w(body+0x14,c['live_maximum'])
 def run(self):
  result=super().run();assert 0x5014f0 not in h.VISITED
  return result

def main():
 rows=[]
 for present in [False,True]:
  for source,current,maximum in [(500.0,125.0,1000.0),(17.25,1.5,99.0),(0.125,0.0625,0.25)]:
   for stop in ['none','stat3','stat5']:
    c={'class':3,'time':0x40800000,'flags':0x40a12400,'position':[0x3fc00000,0xc0200000],'variant':17,'owner':781,'display':'Farm','key':'','text':'','kingdom':'','area':'','camera':'','stop':stop,'data_variations':True,'source_health':h.f32(source),'current_health':h.f32(current),'live_maximum':h.f32(maximum),'body_present':present}
    for pc in h.CW:
     out=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert out[0]==out[1]
     if stop=='none':assert out[0]['stats']==[c['current_health']if present else 0,c['live_maximum']if present else 0,0,0,c['source_health']if present else 0]
     rows.append({'input':c,'pc':pc,'native':out[0]})
 static=[{'va':hex(a),'bytes':h.P.range(a,n).hex(),'meaning':label}for a,n,label in [(0x466314,24,'464BD0 health fixed getter then fstp Data+284'),(0x4cec90,29,'4CEC90 pushesNULL modifier and has actual absentBody zero leaf'),(0x529c30,7,'Body+4 owner then owner+4 Data'),(0x45d730,34,'NULL modifier sourceData+284 path, positive5014F0 not used here')]]
 for x in static:assert h.P.range(int(x['va'],16),len(bytes.fromhex(x['bytes'])))==h.V.range(int(x['va'],16),len(bytes.fromhex(x['bytes'])))
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':v,'kind':'executed_or_explicit_required_stat_entry'}for a,v in sorted(h.VISITED.items())],'static_source_pins':static,'positive_modifier_5014F0_unreached':True}
 p=HERE/'native-memory-body-source.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
