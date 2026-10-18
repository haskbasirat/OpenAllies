"""Fresh actual ActorStore GTID28 input, actual4EC4A0 output compared with Rust record.
No computed Rust output is reused as an input. Name/pose/flags/variant/time come
from the pre-refresh actual input recorder. Data.byte108 is tested both ways:
Doodad actual Data.virtual40 is true, so neither case invents its source value.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_memory_refresh import Machine,h,H

def main():
 source=HERE/'actual-staged-refresh.json';actual=json.loads(source.read_text());i=actual['actual_input'];assert i['class']==9
 rows=[]
 display=bytes(b for unit in i['display']for b in int(unit).to_bytes(2,'little')).decode('utf-16-le')
 for variation in [False,True]:
  c={'class':9,'time':i['time'],'flags':i['flags'],'position':i['position'],'variant':i['variant'],'owner':0,'display':display,'key':'','text':'','kingdom':'','area':'','camera':'','stop':'none','data_variations':variation}
  for pc in h.CW:
   values=[Machine(pc,image,c).run()for image in(h.P,h.V)];assert values[0]==values[1];v=values[0]
   record=actual['first_memory'][0]
   assert v['time']==i['time']and v['outcome']=='complete'and v['definition']
   expected={'name':list(v['name'].encode('utf-16-le')),'position':v['position'],'owner':v['owner'],'aspect':v['aspect'],'stats':v['stats'],'flags':v['flags']}
   #CString comparison remains exact UTF16 code units, not Unicode-normalized text.
   name=[int.from_bytes(bytes(expected['name'][n:n+2]),'little')for n in range(0,len(expected['name']),2)];assert name==record['name']
   for key in ['position','owner','aspect','stats','flags']:assert v[key]==record[key],(key,v,record)
   assert record['definition']==i['definition']['id']
   rows.append({'pc':pc,'explicit_data_variations_guard':variation,'native':v})
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'actual_sha256':H(source.read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'input':i,'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':b,'kind':'executed'}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-actual-memory-refresh.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':doc['machines'],'pins':len(doc['pins']),'fixture_sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
