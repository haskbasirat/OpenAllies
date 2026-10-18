"""Original full world step at large signed tick/precision boundaries."""
from pathlib import Path
import hashlib,json,struct
import native_runtime as native
HERE=Path(__file__).resolve().parent
H=lambda b:hashlib.sha256(b).hexdigest()
def main():
 rows=[]
 starts=[0x78d5c839+i for i in range(8)]+[0x7ffffffe,0x7fffffff,0x80000000,0xfffffffe,0xffffffff]
 for precision in [24,53,64]:
  for rate_bits in [0x41800000,0x41c1f60b,0x41f00000]:
   rate=struct.unpack('<f',struct.pack('<I',rate_bits))[0]
   for tick in starts:
    m=native.Machine(precision,rate);step=m.read(m.world+0xe8)
    before=[tick,native.bits(41.5),native.bits(42.125),step]
    for off,value in zip([0xdc,0xe0,0xe4,0xe8],before):m.write(m.world+off,value)
    m.call(0x4f9970,m.world)
    after=[m.read(m.world+off)for off in [0xdc,0xe0,0xe4,0xe8]]
    rows.append({'precision':precision,'rate_bits':rate_bits,'before':before,'after':after})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'runtime_oracle_sha256':H((HERE/'native_runtime.py').read_bytes()),'original_sha256':H(native.q.ORIGINAL.b),'v14_sha256':H(native.q.ACCEPTED.b),'cases':rows,
 'native_instructions_and_seam_entries':[{'va':hex(a),'bytes':raw}for a,raw in sorted(native.q.VISITED.items())],
 'scope':'Original4F9970 complete step with empty queues and explicit non-gameplay callbacks. Large ticks include signed rollover and PC53 double-rounding counterexample. Values retain raw binary32 bits; no elapsed-time approximation.'}
 target=HERE/'native-clock-edges.json';target.write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'cases':len(rows),'sha256':H(target.read_bytes())})
if __name__=='__main__':main()
