from pathlib import Path
import sys,json
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-restoration'));import native_tgi_parser as parser
import trace_registry as T
from unicorn.x86_const import UC_X86_REG_FPCW
for image in [T.P,T.V]:
 T.P=image
 m=parser.Machine(parser.lines_of(b'[SVars]\n{\n DenizenResupplyRateUnderSiege = 0.25\n SupplyProvideUnderSiege = true\n DisableSAI = false\n}\n'))
 result=m.parse();print(json.dumps(result))
 root=m.r(m.r(m.stack+4))if False else None
