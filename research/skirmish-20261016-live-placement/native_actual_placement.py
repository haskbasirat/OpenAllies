"""Untouched 4F4590 -> full4F3640 on actual restored first-Kingdom inputs.
Actual source vtables/virtual bodies execute; all placement child seams are
forbidden. Captured source fields come from NativeSourceRequirements' guarded
owner. This proves isolated placement, not prior initialization/publication.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
ORIGINAL=h.P
H=lambda b:hashlib.sha256(b).hexdigest()
FORBIDDEN={0x4f1660,0x4f0ae0,0x4f2220,0x4f0a70,0x4f0b70}
def run(c,image):
    h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(c['precision'])
    d=m.alloc(0x600);o=m.alloc(0x200);world=m.alloc(0x600);settings=m.alloc(0x200);map_=m.alloc(0x100)
    m.w32(0xb36534,world);m.w32(world+0x28,settings);m.w32(settings+0xa8,c['settings_mode']);m.w32(world+0x30,map_)
    m.w32(map_+4,0,c['cell_size'],*c['extents'])
    m.w32(d,0xa05540);m.w32(d+0x1c,c['definition_flags']);m.w8(d+0x34,c['settlement_space'])
    m.w32(d+0x38,*c['shape']);m.w32(d+0x174,c['component_mask']);m.w8(d+0x1a8,c['source_byte1a8'])
    m.w32(d+0x430,c['source_word430']);m.w32(d+0x444,c['source_word444']);assert c['source_required']is None
    m.w32(d+0x4e0,0);m.w32(o,0xa299d8,d,c['runtime_flags']);m.w32(o+0x20,*c['position']);m.w32(o+0x2c,*c['facing']);m.w32(o+0xe8,c['owner_word'])
    allocator=m.alloc(0x20);vt=m.alloc(0x20);header=m.alloc(0x20)
    m.w32(allocator,vt);m.w32(header,allocator,0,16,1)
    name_events=[];gacd=None;before=None
    def stub(fn):
        a=m.stub_next;m.stub_next+=16;m.u.mem_write(a,b'\xc3');m.seams[a]=fn;return a
    def alloc(_):assert m.arg()==2;m.ret(0,allocator)
    def empty(_):name_events.append('empty');m.ret(0,header)
    def release(_):assert m.arg()==header;name_events.append('release');m.ret(4,0)
    m.seams[0x856370]=alloc;m.w32(vt+0xc,stub(empty));m.w32(vt+4,stub(release))
    objbefore=bytes(m.u.mem_read(o,0x200));defbefore=bytes(m.u.mem_read(d,0x600))
    def observe(u,a,n,_):
        nonlocal gacd,before
        assert a not in FORBIDDEN,hex(a)
        if a==0x4f3640:
            gacd=m.arg();before=bytes(m.u.mem_read(gacd,0x53))
            assert [m.arg(i) for i in range(1,7)]==[o,o,0,0,0,0]
            assert m.r32(gacd+4)==d
            assert m.r32(gacd+8,3)==c['position']
            assert m.r32(gacd+0x14,2)==c['facing'][:2]
            assert m.r32(gacd+0x28)==c['owner_word']
            assert m.r8(gacd+0x52)==((~c['runtime_flags'])>>10)&1
    m.u.hook_add(UC_HOOK_CODE,observe)
    result=m.call(0x4f4590,world,[o,0,0,0,0]);assert m.fpu_top()==0;assert m.final_esp==h.STACK+24
    assert name_events==['empty','release'];assert m.r32(header+0xc)==0
    assert bytes(m.u.mem_read(gacd,0x53))==before
    assert bytes(m.u.mem_read(o,0x200))==objbefore
    assert bytes(m.u.mem_read(d,0x600))==defbefore
    return dict(result=result,descriptor_unchanged=True,object_unchanged=True,source_unchanged=True,required_child_calls=0,name_events=name_events)
def main():
    capture=HERE/'actual-kingdom-inputs.json';inputs=json.loads(capture.read_text())
    rows=[]
    for c in inputs['cases']:
        a=run(c,'original');b=run(c,'v14');assert a==b;assert a['result']==c['result']
        rows.append(dict(input=c,output=a))
    result=dict(schema=1,scope=__doc__,capture_sha256=H(capture.read_bytes()),original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),
                cases=rows,native_executions=len(rows)*2,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-actual-placement.json'
    if '--write'in sys.argv:path.write_text(json.dumps(result,indent=1)+'\n')
    else:assert json.loads(path.read_text())==result
    print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()
