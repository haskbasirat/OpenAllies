"""Original/v14 45F240 snapshot-list insertion chronology.

Only seam is operator new6F7810: return a fresh 8-byte fixture allocation
filled with A5. All link writes, input-value reads and head/tail updates
execute original code. Valid initial lists have matching head/tail identity.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine,sha,instructions
ORIGINAL,V14=nm.original,nm.modified

def run(exe,initial,inserts):
    nm.original=exe;m=Machine('Double53'); header=m.alloc(8);nodes=[]
    for value in initial:
        node=m.array([value,0])
        if nodes:m.w(nodes[-1]+4,node)
        nodes.append(node)
    m.w(header,nodes[0] if nodes else 0);m.w(header+4,nodes[-1] if nodes else 0)
    original_head=m.r(header);allocations=[]
    def allocate(mm):
        count=mm.arg(1);assert count==8
        node=mm.alloc(count);mm.u.mem_write(node,b'\xa5'*count)
        allocations.append({'size':count,'node':node});mm.ret(node)
    m.seam(0x6f7810,allocate)
    cell=m.alloc(4);steps=[]
    for k,value in enumerate(inserts):
        m.w(cell,value);m.call(0x45f240,header,[cell])
        assert len(allocations)==k+1
        observed=[];values=[];cursor=m.r(header)
        while cursor:
            assert cursor not in observed
            observed.append(cursor);values.append(m.r(cursor));cursor=m.r(cursor+4)
        expected=initial+inserts[:k+1]
        assert values==expected
        tail=m.r(header+4);new_node=allocations[-1]['node']
        assert tail==observed[-1]==new_node
        assert m.r(tail+4)==0
        if original_head:assert m.r(header)==original_head
        else:assert m.r(header)==allocations[0]['node']
        steps.append({'inserted':value,'values_head_to_tail':values,'nodes_head_to_tail':observed,
                      'head':m.r(header),'tail':tail,'allocated_node':new_node,'tail_next':m.r(tail+4),
                      'input_cell_unchanged':m.r(cell)==value})
    for a,b in m.visited.items():assert ORIGINAL.range(a,len(b))==b==V14.range(a,len(b)),hex(a)
    return {'steps':steps,'allocations':allocations},m.visited

def main():
    rows=[];visited={};executions=0
    cases=[([], [0x11111111,0x22222222,0x11111111]),
           ([0x33333333],[0x11111111,0x22222222,0x11111111]),
           ([0x33333333,0x44444444],[0x11111111,0x22222222,0x11111111]),
           ([7,7],[7,0,7]),([], [0,0,0xffffffff])]
    for initial,inserts in cases:
        outputs=[]
        for exe in [ORIGINAL,V14]:
            result,seen=run(exe,initial,inserts);outputs.append(result);visited.update(seen);executions+=len(inserts)
        assert outputs[0]==outputs[1]
        rows.append({'initial':initial,'insertions':inserts,'expected':outputs[0]})
    nm.original=ORIGINAL
    out={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),
         'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'native_calls':executions,
         'instructions':instructions(visited),'seams':[{'address':'0x6f7810','kind':'fresh8byteallocator','calling_convention':'cdecl'}]}
    path=HERE/'snapshot-list.json';path.write_text(json.dumps(out,indent=2)+'\n')
    print(f'{len(rows)} list scenarios; {executions} original/v14 native insertions; {len(visited)} pinned instructions')
    print(path,sha(path.read_bytes()))

if __name__=='__main__':main()
