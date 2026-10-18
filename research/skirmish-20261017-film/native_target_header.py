"""Actual fresh5A28B0 CString literal/caller operands and5A2A40 post-read
name gate/raw extent copies. The factory fragment stops before manager
publication; its5A27A0 child is a required argument-capture seam. Postread
uses real4981A0/511860 and405B80; Creator load is supplied explicit storage,
not declared a successful polymorphic resource read. No full MapResourceLoad.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def factory(image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);frame=L.STACK+0x4000
 source=m.alloc(0x70);m.w(frame+8,source);m.u.reg_write(UC_X86_REG_EBP,frame);m.u.reg_write(UC_X86_REG_ESP,L.STACK+0x8000)
 m.u.emu_start(0x5a28d1,0x5a28de);out={}
 def ctor():
  out.update(ids=[ord(c)for c in m.text(m.r(m.arg(0)))],same_creator=m.arg(1)==source,flag=m.arg(2));m.ret(12,m.u.reg_read(UC_X86_REG_ECX))
 m.seam(0x5a27a0,ctor);m.u.reg_write(UC_X86_REG_EAX,0);m.u.emu_start(0x5a2998,0x5a29ee)
 assert out==dict(ids=[110,101,119],same_creator=True,flag=1)
 assert ORIGINAL.at(0xa44368,8)==EXPANDED.at(0xa44368,8)==b'n\0e\0w\0\0\0'
 return out,m.windows()
def caller(image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);frame=L.STACK+0x4000
 source=m.alloc(0xb4);target=m.alloc(0xb4);m.w(frame-0x1c,source);events=[]
 def request():
  assert m.u.reg_read(UC_X86_REG_ECX)==source and m.arg(0)==target+0x3c
  events.append('source_request');m.ret(4)
 def load():
  assert m.u.reg_read(UC_X86_REG_ECX)==target and m.arg(0)==source
  events.append('target_read_with_source_argument');m.ret(4)
 m.seam(0x5aa2e0,request);m.seam(0x5a2a40,load)
 m.u.reg_write(UC_X86_REG_EBP,frame);m.u.reg_write(UC_X86_REG_ESI,target);m.u.reg_write(UC_X86_REG_ESP,L.STACK+0x8000)
 m.u.emu_start(0x5a2d09,0x5a2d1f)
 return events,m.windows()
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc])
 arr=m.alloc(28);m.w(0xb3b198,arr)
 def register():
  entry=m.alloc(12);m.w(entry+8,m.arg(2));m.w(arr+4*m.arg(0),entry);m.ret(12,entry)
 m.seam(0x4976b0,register);m.call(0x4981a0,m.alloc(32))
 resource=m.alloc(0xb4);source=m.alloc(0xb4);header=m.alloc(0x68);rules=m.alloc(0x28);creator=m.alloc(0x70)
 m.w(source+0x38,m.cstring(case['source_ids']))
 m.w(resource+0x38,m.cstring('new'));m.w(resource+0x90,header);m.w(resource+0x88,creator)
 m.w(header+4,rules);m.w(rules+4,case['mode']);m.w(header+0x38,m.cstring(case['name']))
 for i,v in enumerate(case['extents']):m.w(creator+0x3c+4*i,v)
 def header_read():
  assert m.u.reg_read(UC_X86_REG_ECX)==header and m.arg(0)==resource+0x3c and m.arg(1)==resource and m.arg(2)==source
  m.ret(12)
 m.seam(0x5a19c0,header_read)
 m.u.reg_write(UC_X86_REG_ESI,resource);m.u.reg_write(UC_X86_REG_EDI,resource+0x3c)
 m.u.reg_write(UC_X86_REG_ESP,L.STACK+0x8000);m.w(L.STACK+0x8024,source)
 m.u.emu_start(0x5a2b04,0x5a2b39)
 m.u.reg_write(UC_X86_REG_ESI,resource);m.u.reg_write(UC_X86_REG_EDI,resource+0x88)
 m.u.reg_write(UC_X86_REG_ESP,L.STACK+0x8000);m.u.emu_start(0x5a2b5e,0x5a2b7c)
 return dict(name=[ord(c)for c in m.text(m.r(header+0x38))],extents=[m.r(header+0x30),m.r(header+0x34)]),m.windows()
def main():
 rows=[];blocks={};factory_rows=[];caller_rows=[]
 for pc in PC:
  a,pa=factory('original',pc);b,pb=factory('v14',pc);assert a==b
  factory_rows.append(dict(pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
  a,pa=caller('original',pc);b,pb=caller('v14',pc);assert a==b
  caller_rows.append(dict(pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 for mode in range(7):
  for name,bits in [('original',[0,0x80000000]),('',[0x7f812345,0xffc54321]),('saved',[0x44800001,0x44400001])]:
   case=dict(mode=mode,name=name,extents=bits,source_ids='Maps/actual_source.RMP')
   for pc in PC:
    a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
    rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,factory_ids=[110,101,119],factory_executions=6,factory_rows=factory_rows,
          caller_rows=caller_rows,caller_executions=6,literal=dict(address='0xa44368',bytes=ORIGINAL.at(0xa44368,8).hex()),cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-target-header.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows)+12,blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
