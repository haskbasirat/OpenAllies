"""Direct6FE050/8742D0 NiCamera child, arbitrary finite local transforms and
perspective/orthographic frusta. No trig/device/scene calls. Parent45CAB0 and
attached+B0 vf38 are explicit stopped required-service boundaries, not
successful substituted children. All other called arithmetic executes.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL=L.IMAGE;MODIFIED=L.ACCEPTED
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
OUTPUTS=[0x28,0x2c,0x30]+list(range(0x6c,0xa0,4))+list(range(0xc8,0x108,4))+[0x10c,0x110]+list(range(0x114,0x138,4))+list(range(0x19c,0x1fc,4))+list(range(0x2d4,0x304,4))
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Required(Exception):pass
def run(image,precision,c):
 L.IMAGE=ORIGINAL if image=='original'else MODIFIED
 m=L.Loader(PCS[precision]);camera=m.alloc(0x394);m.u.mem_write(camera,b'\xcd'*0x394);m.w(camera,0xa84850)
 m.w(camera+0x24,1 if c.get('gate')=='parent'else 0);m.w(camera+0xb0,0)
 for i,v in enumerate(c['local']):m.w(camera+0x38+4*i,v)
 for i,v in enumerate(c['frustum']):m.w(camera+0x138+4*i,v)
 m.u.mem_write(camera+0x150,bytes([c['ortho']]))
 if c.get('gate')=='parent':m.seam(0x45cab0,lambda:(_ for _ in()).throw(Required('parent')))
 if c.get('gate')=='attached':
  observer=m.alloc(4);vt=m.alloc(0x3c);m.w(observer,vt);m.w(vt+0x38,L.STUBS+0x600);m.w(camera+0xb0,observer)
  m.seam(L.STUBS+0x600,lambda:(_ for _ in()).throw(Required('attached')))
 writes=[]
 def write(u,access,address,size,value,_):writes.append({'pc':hex(u.reg_read(UC_X86_REG_EIP)),'offset':address-camera,'size':size,'value':value})
 m.u.hook_add(UC_HOOK_MEM_WRITE,write,begin=camera,end=camera+0x393)
 error=None
 try:m.call(0x6fe050,camera)
 except Required as e:error=str(e)
 pins=[{'address':hex(a),'bytes':L.IMAGE.at(a,n).hex(),'sha256':hashlib.sha256(L.IMAGE.at(a,n)).hexdigest()}for a,n in sorted(m.visited.items())]
 return {'error':error,'words':[m.r(camera+o)for o in OUTPUTS],'writes':writes},pins
def cases():
 matrices=[
  [1,0,0,0,1,0,0,0,1],
  [0,0,0,0,0,0,0,0,0],
  [0,-1,0,1,0,0,0,0,-1],
  [1.125,0.0625,-0.25,-0.125,0.875,0.5,0.25,-0.625,1.5],
  [0.0001,0,0,0,0.0001,0,0,0,0.0001],
  [-0.0,0.33333334,0.99999994,-0.99999994,-0.0,0.33333334,0.33333334,-0.99999994,-0.0],
 ]
 frusta=[[-1,1,1,-1,1,100],[-0.01,0.02,0.03,-0.025,0.1,512],[-2,3,4,-5,0.25,20],[-0.0001,0.0002,0.0003,-0.00025,0.001,1000]]
 for i,(matrix,frustum,ortho)in enumerate(itertools.product(matrices,frusta,[0,1])):
  local=list(map(bits,matrix+[10.125,-7.75,3.0625]))+[0x7fc01234 if i%2 else 0x80000000]
  yield {'local':local,'frustum':list(map(bits,frustum)),'ortho':ortho,'gate':None}
 base={'local':list(map(bits,matrices[0]+[10,-7,3]))+[bits(1)],'frustum':list(map(bits,frusta[0])),'ortho':0}
 for gate in ['parent','attached']:yield dict(base,gate=gate)
def main():
 rows=[];pins={}
 for index,c in enumerate(cases()):
  for pc in PCS:
   a,pa=run('original',pc,c);b,pb=run('v14',pc,c);assert a==b,(index,pc)
   rows.append({'case':index,'precision':pc,'input':c,'output':a});pins.update({v['address']:v for v in pa+pb})
 for p in pins.values():
  at=int(p['address'],16);raw=bytes.fromhex(p['bytes']);assert ORIGINAL.at(at,len(raw))==MODIFIED.at(at,len(raw))==raw
 constants={hex(a):ORIGINAL.at(a,4).hex()for a in [0x9fd38c,0x9f32e0,0x9fdc48,0x9f32dc]}
 for a,h in constants.items():assert MODIFIED.at(int(a,16),4).hex()==h
 for offset,target in [(0x64,0x873790),(0x68,0x8742d0)]:
  assert ORIGINAL.at(0xa84850+offset,4)==MODIFIED.at(0xa84850+offset,4)==struct.pack('<I',target)
 assert all(not any(0x164<=w['offset']<0x168 for w in r['output']['writes'])for r in rows)
 out={'schema':1,'scope':__doc__,'output_offsets':OUTPUTS,'rows':rows,'blocks':[pins[k]for k in sorted(pins)],'constants':constants,'vtable':{'address':'0xa84850','slot64':ORIGINAL.at(0xa84850+0x64,4).hex(),'slot68':ORIGINAL.at(0xa84850+0x68,4).hex()}}
 path=HERE/'native-camera-ni.json';raw=(json.dumps(out,indent=1)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps({'paired_rows':len(rows),'native_runs':2*len(rows),'pins':len(pins),'fixture_sha256':hashlib.sha256(raw).hexdigest()}))
if __name__=='__main__':main()
