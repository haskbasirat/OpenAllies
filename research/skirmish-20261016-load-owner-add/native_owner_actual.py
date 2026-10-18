"""Re-execute4E6180 prefix over the actual source/captured Rust held-record inputs."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_owner_add as n
H=lambda b:hashlib.sha256(b).hexdigest()
class Actual(n.Machine):
 def __init__(self,pc,c):
  super().__init__(pc);self.actor_id=c['actor_id'];self.ids[self.a]=self.actor_id
  head=0;tail=0
  for actor_id in reversed(c['kingdom_supply']):
   actor=self.a if actor_id==self.actor_id else self.alloc(0x200)
   self.ids[actor]=actor_id;self.w32(actor+0x14,actor_id)
   node=self.alloc(8);self.w32(node,actor,head);head=node
   if tail==0:tail=node
  self.w32(self.k+0x264,head,tail)
 def w32(self,address,*values):
  if hasattr(self,'actor_id') and address==self.a+0x14:values=(self.actor_id,*values[1:])
  return super().w32(address,*values)
def main():
 source=HERE/'actual-owner-inputs-v2.json';actual=json.loads(source.read_text());rows=[]
 assert len(actual['cases'])==3
 for img,label in [(n.P,'original'),(n.V,'v14')]:
  n.h.P=img
  for row in actual['cases']:
   pc=row['precision'];c=row['input'];assert c['economy'] is True
   output=Actual(pc,c).run(c)
   assert output==row['output'],(pc,output,row['output'])
   rows.append({'image':label,'precision':pc,'input':c,'output':output})
 n.h.P=n.P
 for a,raw in n.h.VISITED.items():assert bytes.fromhex(raw)==n.P.range(a,len(bytes.fromhex(raw)))==n.V.range(a,len(bytes.fromhex(raw)))
 out={'schema':1,'original_sha256':n.h.ORIGINAL_SHA,'v14_sha256':n.h.V14_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'helper_sha256':H((HERE/'native_owner_add.py').read_bytes()),'captured_rust_inputs_sha256':H(source.read_bytes()),'cases':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(n.h.VISITED.items())],'scope':'Actual captured current Actor/storedKingdom initial list/flag prefix at all3PC, bothPE; reached actual5704C0 entry is a required effect stop. No successful Economy body, fullInit, fullypublishedWorld, or battle claim. Initial list/capital/vector witnesses are asserted from real owned source capture test, not supplied as fabricated gameplay defaults.'}
 p=HERE/'native-actual-owner.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'cases':len(rows),'pins':len(n.h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
