"""Isolated507C2C..507CAF actual Supply SVar declarations.
Real4017D0/401710 execute with native CString support.4B3470 registration is
an explicit transport capturing constructor metadata; it is not a full Settings
registry proof. Captured variable allocations are laboratory inputs for actual
41C670/40E0A0 raw setters. Actual private TGI/source-owner validation is Rust.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Done(Exception):pass
def lookup(self):self.ret(4,self.supply_cell)
L.Loader._var=lookup

def run(image,pc,bool_value,rate_value):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);rows=[];pointers={}
 def register():
  index=m.arg(0);v=m.arg(1);p=m.r(v+0x1c);chars=[]
  while True:
   ch=int.from_bytes(m.u.mem_read(p+2*len(chars),2),'little')
   if not ch:break
   chars.append(ch)
  name=''.join(map(chr,chars))
  value=m.r(v)&(255 if index==63 else 0xffffffff)
  rows.append(dict(index=index,name=name,value=value,kind=m.r(v+0x24),flags=m.r(v+0x28)))
  copied=m.alloc(0x30);m.u.mem_write(copied,bytes(m.u.mem_read(v,0x2c)));pointers[name]=copied;m.ret(8)
 m.seam(0x4b3470,register)
 def before(u,at,size,_):
  if at==0x507caf:raise Done()
 m.u.hook_add(UC_HOOK_CODE,before)
 m.u.reg_write(UC_X86_REG_ESI,m.alloc(0x40));m.u.reg_write(UC_X86_REG_EDI,0xffffffff)
 try:m.call(0x507c2c)
 except Done:pass
 assert len(rows)==2
 #Named lookup is explicitly transport to the captured actual constructor Var.
 cell=m.alloc(4);m.w(cell,pointers['SupplyProvideUnderSiege']);m.supply_cell=cell
 m.call(0x41c670,0xb39aa8,[m.cstring('SupplyProvideUnderSiege'),bool_value])
 m.call(0x40e0a0,pointers['SupplyStockpileRechargeRate'],[rate_value])
 return dict(constructors=rows,boolean=m.r(pointers['SupplyProvideUnderSiege'])&255,
  rate=m.r(pointers['SupplyStockpileRechargeRate'])),m.windows()

def main():
 rows=[];pins={}
 for value,rate in [(0,0),(128,0x80000000),(255,0x7f800001),(1,0x3f800000)]:
  for pc in PC:
   a,pa=run('original',pc,value,rate);b,pb=run('v14',pc,value,rate);assert a==b
   assert a['constructors']==[dict(index=63,name='SupplyProvideUnderSiege',value=0,kind=1,flags=0),
    dict(index=64,name='SupplyStockpileRechargeRate',value=0x3f800000,kind=2,flags=0)]
   assert a['boolean']==value and a['rate']==rate
   rows.append(dict(pc=pc,input=dict(boolean=value,rate=rate),output=a));pins.update({v['address']:v for v in pa+pb})
 source=ROOT.parent/'reconstruction/assets/original/Data/Game/SVars.tgi'
 text=source.read_text();assert 'SupplyProvideUnderSiege = false'in text and 'SupplyStockpileRechargeRate = 1.0'in text
 out=dict(schema=1,scope=__doc__,rows=rows,blocks=[pins[k]for k in sorted(pins)],source=dict(path=str(source),sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
  declarations=['flag SupplyProvideUnderSiege = false','fixed SupplyStockpileRechargeRate = 1.0']))
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-supply-variables.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
