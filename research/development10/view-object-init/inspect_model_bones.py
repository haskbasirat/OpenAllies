from pathlib import Path
import sys,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import PE0
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
md=Cs(CS_ARCH_X86,CS_MODE_32)
for name in ['bone_name','selection_box']:
 needle=name.encode('utf-16le')+b'\0\0';at=PE0.b.find(needle)
 print(name,'raw',hex(at))
 va=next(PE0.base+section[1]+at-section[3] for section in PE0.sections if section[3]<=at<section[3]+section[2])
 print('VA',hex(va));pointer=struct.pack('<I',va);pos=PE0.b.find(pointer)
 while pos!=-1:
  try:code=next(PE0.base+section[1]+pos-section[3]for section in PE0.sections if section[3]<=pos<section[3]+section[2])
  except StopIteration:pos=PE0.b.find(pointer,pos+1);continue
  print('ref',hex(code))
  for x in md.disasm(PE0.range(code-15,50),code-15):print(hex(x.address),x.bytes.hex(),x.mnemonic,x.op_str)
  pos=PE0.b.find(pointer,pos+1)
