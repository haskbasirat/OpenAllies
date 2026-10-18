from pathlib import Path
import sys
sys.path.insert(0,str(Path.cwd()/'research/phase7-definitions'))
from probe_settings import Machine
m=Machine();s=m.alloc(0x80);m.invoke(0x4b2ce0,s,[])
var=m.alloc(0x2c);m.invoke(0x401710,var,[m.string(list(map(ord,'CombatValueRangePower'))),0x3dcccccd,0]);m.invoke(0x4b3470,s,[0,var])
node=m.node('SVars');parser=m.alloc(0x80);token=m.alloc(0x30);m.w(parser,node);m.w(parser+0x40,m.string(list(map(ord,'CombatValueRangePower'))));m.w(parser+0x38,4);m.w(token+0x18,m.string(list(map(ord,'0.15'))));m.w(token+0x10,7)
try:
 m.invoke(0x5b9e20,parser,[token]);m.invoke(0x4b3c10,s,[node]);print('CV actual fixed=',hex(m.r(m.r(m.r(s+8)))))
except Exception as e: print('ERROR',e,[hex(x) for x in m.recent]);raise

