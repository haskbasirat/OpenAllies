from pathlib import Path
import sys
sys.path.insert(0,'research/phase5-actors');from original_creation import P
from capstone import *
cs=Cs(CS_ARCH_X86,CS_MODE_32)
def text(p):
 try:
  b=P.range(p,200);return b.decode('utf-16-le').split('\0')[0]
 except:return ''
a=int(sys.argv[1],16);z=int(sys.argv[2],16)
for i in cs.disasm(P.range(a,z-a),a):
 s=''
 if i.mnemonic=='push' and i.op_str.startswith('0x'):
  p=int(i.op_str,16)
  if 0x9f0000<p<0xb00000:s=' '+repr(text(p))
 print(f'{i.address:08x}: {i.mnemonic:8} {i.op_str}{s}')
