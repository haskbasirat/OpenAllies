"""Read-only Denizen dispatch and scheduler-admission proof. Actual callback
6A71D0 executes; vtables, zero constant and getter pointers are paired static
PE pins. This does not execute a positive Zone or queue child body.
"""
from pathlib import Path
import json,hashlib,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
O=L.IMAGE;V=L.ACCEPTED
def main():
 pins=[]
 for address,length,label in [(0xa3c124,20,'real Denizen embedded Updatable table; +8 callback'),(0xa3c14c,24,'real Denizen component InitExisting and AfterInit'),(0x6a71d0,3,'true AL leaf'),(0x9f32dc,4,'native guard comparison zero')]:
  raw=O.at(address,length);assert raw==V.at(address,length);pins.append(dict(address=address,length=length,scope=label,bytes_hex=raw.hex(),sha256=hashlib.sha256(raw).hexdigest()))
 def word(a):return struct.unpack('<I',O.at(a,4))[0]
 assert word(0xa3c124+8)==0x6a71d0 and word(0xa3c14c+8)==0x5687e0 and word(0xa3c14c+20)==0x568070
 assert word(0x9f32dc)==0 and O.at(0x6a71d0,3)==b'\xb0\x01\xc3'
 rows=[]
 for name,image in [('original',O),('v14',V)]:
  L.IMAGE=image
  for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
   m=L.Loader(cw);returned=m.call(0x6a71d0);assert returned&255==1;rows.append(dict(image=name,pc=pc,low_return_byte=returned&255))
 raw=(json.dumps(dict(scope=__doc__,pins=pins,rows=rows),indent=1)+'\n').encode();p=HERE/'native-denizen-dispatch.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),static_windows=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
