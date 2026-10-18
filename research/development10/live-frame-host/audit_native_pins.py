"""Read-only original/v14 byte equality and instruction boundary audit.
An executed window is a union of observed instructions, not a claim that
every scenario executes every window. Declared cutoff observer entries are
excluded from the producer's executed census.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import PE0,PE1
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def sha(v):return hashlib.sha256(v).hexdigest()
def main():
 decoder=Cs(CS_ARCH_X86,CS_MODE_32);reports=[]
 for p in [HERE/'native-frame-variables.json',HERE/'native-command-queue.json',HERE/'native-order-gate.json',HERE.parent/'current-svar-sync/native-current-svar.json']:
  f=json.loads(p.read_bytes());count=0;size=0
  for pin in f.get('pins',[]):
   address=int(pin['va'],16);raw=bytes.fromhex(pin['bytes'])
   assert raw==PE0.range(address,len(raw))==PE1.range(address,len(raw))
   instructions=list(decoder.disasm(raw,address));assert len(instructions)==1 and instructions[0].size==len(raw)
   count+=1;size+=len(raw)
  for window in f.get('executed_windows',[]):
   start,end=int(window['start'],16),int(window['end'],16);raw=PE0.range(start,end-start)
   assert raw==PE1.range(start,end-start)and sha(raw)==window['sha256']
   instructions=list(decoder.disasm(raw,start));assert sum(i.size for i in instructions)==len(raw)
   count+=len(instructions);size+=len(raw)
  reports.append(dict(fixture=p.relative_to(ROOT).as_posix(),sha256=sha(p.read_bytes()),instructions=count,bytes=size))
 print(json.dumps(dict(passed=True,original_sha256=sha(PE0.b),v14_sha256=sha(PE1.b),reports=reports),indent=2))
if __name__=='__main__':main()
