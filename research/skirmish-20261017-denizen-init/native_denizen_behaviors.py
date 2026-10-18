"""Real463520 behavior tokenizer/keyword OR with one declared typed String
getter5B54D0. Original CString/tokenizer/CRT comparisons execute. Unknown input
stops at native410C40 diagnostics. Organization grouping/append and source
flag stores are separately static-byte pinned; no full DenizenData loader,
deferred reference graph, group size or formation constructor claim.
"""
from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass
def run(text,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc])
 value=m.alloc(4);m.w(value,m.cstring(text));definition=m.alloc(0x20);m.w(definition+8,m.cstring('explicit-source-transport'))
 m.seam(0x5b54d0,lambda:m.ret(4,value))
 def diagnostic():raise Stop('native_unknown_behavior')
 m.seam(0x410c40,diagnostic)
 try:out={'flags':m.call(0x463520,0,[0,definition]),'stopped':None}
 except Stop as e:out={'flags':None,'stopped':str(e)}
 return out,m.windows()
def main():
 names=['worker','militia_offensive','militia_defensive','air_offensive','air_defensive','air_recon','air_escort']
 texts=['',' + ',' + '.join(names),'WORKER+air_recon+WORKER','air_escort  +  militia_defensive','worker\tair_recon','unknown']+names+[s.upper()for s in names]
 rows=[];pins={}
 for text in texts:
  for pc in PC:
   a,pa=run(text,'original',pc);b,pb=run(text,'v14',pc);assert a==b
   rows.append({'input':text,'pc':pc,'output':a});pins.update({v['address']:v for v in pa+pb})
 static=[]
 for a,b,label in [(0x467165,0x467350,'Organization case-sensitive grouping then DenizenGroup append'),(0x463740,0x463800,'Organization flags at node8; zero behavior diagnostic'),(0x463800,0x463a60,'DenizenGroup flags at node4; positive group other prerequisites remain separate'),(0x45f820,0x45f8cb,'DenizenGroup append to tail')]:
  raw=ORIGINAL.at(a,b-a);assert raw==EXPANDED.at(a,b-a);static.append({'start':a,'end':b,'sha256':hashlib.sha256(raw).hexdigest(),'scope':label})
 raw=(json.dumps({'scope':__doc__,'cases':rows,'blocks':[pins[k]for k in sorted(pins)],'static':static},indent=1)+'\n').encode();p=HERE/'native-denizen-behaviors.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'pins':len(pins),'static_windows':len(static),'sha256':hashlib.sha256(raw).hexdigest()}))
if __name__=='__main__':main()
