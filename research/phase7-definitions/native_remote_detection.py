from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_ground_definitions import Machine,P,V,H,bits
from unicorn.x86_const import UC_X86_REG_EBP
rows=[];visited={}
for flags in [0,1<<7,1<<22,(1<<7)|(1<<22)]:
 for remote in [-3.,-0.,0.,2.,40.]:
  m=Machine();p=m.definition;m.w(p+0x174,flags);m.w(p+0x1d0,bits(remote));m.w(p+0x2fc,bits(18))
  m.run(0x46889e,0x4688d4,[(UC_X86_REG_EBP,p)])
  value=m.r(p+0x1d0);assert value==(bits(18)if flags==(1<<7)|(1<<22) and remote>=0 else bits(remote))
  rows.append({'flags':flags,'remote_bits':bits(remote),'control_bits':bits(18),'output_bits':value});visited.update(m.visited)
pins=[]
for a,b in [(0x46889e,0x4688d4),(0x4624b0,0x462d50)]:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
p=HERE/'native-remote-detection.json';p.write_text(json.dumps({'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'ranges':pins,'scope':'Original remote range conditional fragment executed. Full ActorData constructor statically pinned for absent component zero-range stores; not complete ctor execution.'},indent=2));print('passed',len(rows))
