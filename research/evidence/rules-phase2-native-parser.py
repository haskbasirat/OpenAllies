"""Unchanged native key-scanner fragment; static parser evidence for remaining gaps."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_EAX
image=(GAME/'AA.exe').read_bytes();native=(GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()
assert hashlib.sha256(image).hexdigest()=='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
start,end=0x5be8c4,0x5be90d
assert image[start-0x400000:end-0x400000]==native[start-0x400000:end-0x400000]
uc=Uc(UC_ARCH_X86,UC_MODE_32);uc.mem_map(0x400000,0xc00000);uc.mem_write(0x400000,image);uc.mem_map(0x2000000,0x20000)
def w(a,v):uc.mem_write(a,struct.pack('<I',v))
def r(a):return struct.unpack('<I',uc.mem_read(a,4))[0]
rows=[]
for key,suffix in [('key_[',' = "["'),('key_]',' = "]"'),('name[variant]',' = value'),('plain','\t = 2'),('mixedUPPER','; comment'),('with-brace{',' = x'),('typed key',' = 0'),('final_name',''),('slash','/comment'),('spaced key','\t;end')]:
 text=key+suffix;utf=text.encode('utf-16-le');p=0x2001000;obj=0x2000000;uc.mem_write(p,utf+b'\0\0');w(p-12,len(text));w(obj+8,p);w(obj+12,0)
 uc.reg_write(UC_X86_REG_ESI,obj);uc.reg_write(UC_X86_REG_EBX,0);uc.reg_write(UC_X86_REG_ESP,0x2018000)
 uc.emu_start(start,end,count=10000)
 length=uc.reg_read(UC_X86_REG_EBX)
 actual=text[:length];assert actual==key,(actual,key)
 rows.append(dict(line=text,key=actual,end_offset=r(obj+12),key_length=length))
funcs=['005bd080','005b6450','005baeb0','005bde30','005be8a0','005bdf80','005bee60','0048a350','0048b800']
evidence=[]
for f in funcs:
 p=GAME/f'reconstruction/research/native-aa/pseudocode/{f}.c';evidence.append(dict(path=str(p.relative_to(GAME)).replace('\\','/'),sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
windows={}
for a,z in [(0x5bd080,0x5bd180),(0x5baeb0,0x5bb050),(0x5bde30,0x5bdf80),(0x5bdf80,0x5be130),(0x5bee60,0x5bef10),(0x48a350,0x48a480),(0x48b800,0x48bb00)]:
 assert image[a-0x400000:z-0x400000]==native[a-0x400000:z-0x400000]
 windows[hex(a)]={'end_exclusive':hex(z),'sha256':hashlib.sha256(image[a-0x400000:z-0x400000]).hexdigest(),'unchanged_in_v14':True}
report=dict(schema=1,static_code_windows=windows,passed=True,original_native_sha256=hashlib.sha256(image).hexdigest(),v14_native_sha256=hashlib.sha256(native).hexdigest(),generator_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),method='Unchanged original x86 key-scanning fragment, no hooks or live game; source pseudo-code is crosschecked against original instructions for the tested range.',range=dict(start=hex(start),end_exclusive=hex(end),sha256=hashlib.sha256(image[start-0x400000:end-0x400000]).hexdigest()),key_scanner=rows,evidence=evidence,conclusions={'bracket_keys':'Native key scanner accepts internal [ and ]; this narrow parser extension is oracle-backed.','missing_header_close':'5BDE30 reads through current line to ] or line end; 5BD080 supplies one line at a time. Whole grammar consequences not yet emulated; no inferred ] inserted.','missing_body_close':'5BD080 calls5BAEB0 after stream EOF; that drains parser stack. Exact parent/child results for malformed sources not yet oracle-tested; strict AST rejection retained.','localized_quote':'The shipped trigger_elements line355 ends with an extra adjacent quote.5BDF80 consumes a line and5BEE60 tests first/last quote or null rather than balanced quotes. Full trim/localization behavior was not executed; Rust strict rejection retained instead of deleting the quote.','delayed_body':'Native token dispatch/body stack effect after intervening properties not yet proven; rejected.'},scope='Ten scanner cases. Not a complete native parser/tokenizer, template evaluator, line-normalization or malformed-source acceptance oracle.')
p=ROOT/'research/evidence/rules-phase2-native-parser.json';p.write_text(json.dumps(report,indent=2)+'\n');print(p,hashlib.sha256(p.read_bytes()).hexdigest())
