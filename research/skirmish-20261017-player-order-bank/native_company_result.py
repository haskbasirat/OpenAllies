"""Real527E30 ground-command caller consumes520440's actual AL result.

Actual destination/clamp/group-coordinate path runs. Existing named state
eligibility/activation and group helper contracts come from the prior company
movement harness; no activation/state-entry/movement result is claimed as a
completed live effect. Debug flag is the fixture's fresh image-zero value.
Each case executes independently in both immutable PEs/all x87 precision modes.
"""
from pathlib import Path
import argparse,json,sys,copy,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from native_company import Company,cases
from unicorn.x86_const import UC_X86_REG_EAX
P=h.P;V=h.V;H=lambda b:hashlib.sha256(b).hexdigest()
class Probe(Company):
 def run_command(self):
  self.regiment();command=self.alloc(0x40)
  x,y,speed,auxx,auxy=self.c['args']
  self.w32(command+0xc,0x5c);self.w32(command+0x18,x,y,auxx,auxy,speed)
  result=self.call(0x527e30,self.mind,[command])&255
  assert self.final_esp==h.STACK+8 and self.fpu_top()==0 and not self.responses
  return {'al':result,'state':self.state_out(),'events':self.events}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args();rows=[]
 selected=[c for c in cases()if c['operation']=='activate_location'and c['mode']==2]
 assert selected
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for c in selected:
    case=copy.deepcopy(c);case['precision']=pc
    out=Probe(case).run_command();rows.append({'image':label,'input':case,'output':out})
 h.P=P
 for n in range(1,6):assert [r['output']for r in rows[n*len(selected):(n+1)*len(selected)]]==[r['output']for r in rows[:len(selected)]]
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'rows':rows,'executed_and_named_callback_entry_pins':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-company-result.json'
 if a.write:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
