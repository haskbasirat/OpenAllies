"""Original high-level search/heap/neighbor execution with final path formatting isolated.

All queue decisions, tile flags, visit stamps, directions, costs and heuristics run
from the original PE. The sole behavioral hook records 53D500's endpoint instead
of materializing its separately owned path/line-of-sight layer.
"""
from pathlib import Path
import hashlib
import json
import struct
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_ECX,UC_X86_REG_EIP,UC_X86_REG_ESP

WORLD,GRID,SOLVER,DESC,PATH=0x4000000,0x4001000,0x4002000,0x4003000,0x4003100
TILES,HEAP,REGISTRY=0x4100000,0x4300000,0x4400000
sha=lambda b:hashlib.sha256(b).hexdigest()
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
RANGES={'search':(0x53d8d0,0x53dbff),'neighbor':(0x53c900,0x53ca25),
        'heap_push':(0x53d090,0x53d147),'heap_pop':(0x53d150,0x53d298),
        'heuristic':(0x53c7a0,0x53c870),'cost':(0x53cb70,0x53cd99),
        'neighbors':(0x554ed0,0x555010),'neighbor_filter':(0x4f2000,0x4f2160),
        'neighbor_next':(0x4f2760,0x4f27c0),'mark':(0x53c6c0,0x53c705),
        'tables':(0xab7804,0xab7904),'offsets':(0xb322d8,0xb32328)}


class SearchMachine(Machine):
    def __init__(self,control_word):
        super().__init__(control_word)
        self.u.mem_map(0,0x1000)
        self.u.mem_map(0x4000000,0x800000)
        self.finished=[]
        self.popped=[]
        self.collision_trace=[]
        self.u.hook_add(UC_HOOK_CODE,self.hook)
    def hook(self,u,address,size,user):
        if address==0x53d500:
            sp=self.rsp()
            self.finished.append([self.signed(self.r(sp+4)),self.signed(self.r(sp+8))])
            strategy=u.reg_read(UC_X86_REG_ECX)
            self.w(strategy+4,0);self.w(strategy+0x1c,0)
            u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+12)
        if address==0x53da99:
            self.popped.append(list(struct.unpack('<5I',u.mem_read(SOLVER+0x24,20))))
        if address==0x54f370:
            index=(u.reg_read(UC_X86_REG_ECX)-TILES)//0x30
            width=self.r(GRID+0x1c)
            tile=TILES+index*0x30
            self.collision_trace.append([index%width,index//width,self.r(tile+0x28),u.mem_read(tile+0x2d,1)[0]])
    def rsp(self):return self.u.reg_read(UC_X86_REG_ESP)
    @staticmethod
    def signed(v):return v-(1<<32) if v>=1<<31 else v
    def run(self,case):
        self.reset(0)
        self.w(0xb36534,WORLD);self.w(WORLD+0x30,GRID)
        self.w(0xb364c0,REGISTRY)
        width,height=case['size']; start=case['start'];goal=case['goal']
        self.w(GRID+4,case.get('shift',1));self.w(GRID+8,bits(case.get('cell_size',2.)))
        self.w(GRID+0xc,bits(width*case.get('cell_size',2.)));self.w(GRID+0x10,bits(height*case.get('cell_size',2.)))
        self.w(GRID+0x1c,width);self.w(GRID+0x20,height);self.w(GRID+0x28,TILES)
        self.w(GRID+0x48,case.get('initial_epoch',0))
        self.u.mem_write(TILES,bytes(width*height*0x30))
        for y in range(height):
            for x in range(width):
                edge=x==0 or y==0 or x==width-1 or y==height-1
                blocked=[x,y]in case.get('blocked',[])
                self.w(TILES+(y*width+x)*0x30+0x20,1 if edge else (2 if blocked else 0))
        self.u.mem_write(SOLVER,bytes(0x200))
        self.w(SOLVER,0xa366a8);self.w(SOLVER+4,GRID)
        strategy=SOLVER+0x44;queue=SOLVER+0x84
        self.w(SOLVER+8,queue);self.w(SOLVER+12,strategy)
        self.w(strategy,0xa36610);self.w(strategy+4,DESC);self.w(strategy+0x1c,PATH)
        for off,value in zip([0xc,0x10,0x14,0x18],start+goal):self.w(strategy+off,value)
        center=lambda p:[v*case.get('cell_size',2.)+case.get('cell_size',2.)*.5 for v in p]
        for off,value in zip([0x20,0x24,0x28,0x2c],center(start)+case.get('goal_world',center(goal))):self.w(strategy+off,bits(value))
        self.w(DESC+0x3c,2)
        self.w(queue,0xa3668c);self.w(queue+4,HEAP);self.w(queue+8,0)
        self.w(queue+12,16384);self.w(queue+16,case.get('queue_limit',0))
        self.u.reg_write(UC_X86_REG_ECX,SOLVER)
        self.u.emu_start(0x53d8d0,self.STOP,count=20000000)
        assert self.rsp()==self.STACK+4
        stamp=self.r(GRID+0x48)
        visits=[]
        for y in range(height):
            for x in range(width):
                a=TILES+(y*width+x)*0x30
                if self.r(a+0x28)==stamp:
                    visits.append({'cell':[x,y],'direction':self.u.mem_read(a+0x2d,1)[0]})
        return {'status':self.u.reg_read(UC_X86_REG_EAX),'finished':self.finished,
                'popped':self.popped,'visit_epoch':stamp,'visited':visits,
                'collision_trace':self.collision_trace,
                'queue_remaining':self.r(queue+8)}


def main():
    for a,b in RANGES.values():assert original.range(a,b-a)==modified.range(a,b-a)
    cases=[
        {'name':'open_diagonal','size':[12,12],'start':[2,2],'goal':[9,9]},
        {'name':'open_oblique','size':[15,12],'start':[2,3],'goal':[12,8]},
        {'name':'same_cell','size':[7,7],'start':[3,3],'goal':[3,3]},
        {'name':'wall_gap','size':[12,12],'start':[2,2],'goal':[9,2],
         'blocked':[[6,y]for y in range(1,9)]},
        {'name':'blocked_goal','size':[12,12],'start':[2,2],'goal':[9,9], 'blocked':[[9,9]]},
        {'name':'boxed_start','size':[7,7],'start':[3,3],'goal':[5,5],
         'blocked':[[x,y]for y in range(2,5)for x in range(2,5)if[x,y]!=[3,3]]},
        {'name':'queue_limit_one','size':[12,12],'start':[2,2],'goal':[9,9],'queue_limit':1},
        {'name':'queue_limit_three','size':[12,12],'start':[2,2],'goal':[9,9],'queue_limit':3},
        {'name':'long_depth_partial','size':[300,3],'start':[1,1],'goal':[298,1]},
        {'name':'long_depth_failure','size':[300,5],'start':[2,1],'goal':[2,3],
         'blocked':[[x,2] for x in range(1,298)]},
        {'name':'near_corner','size':[10,10],'start':[1,1],'goal':[8,8],
         'blocked':[[2,1],[1,2]]},
        {'name':'equal_routes','size':[11,11],'start':[2,5],'goal':[8,5],
         'blocked':[[5,y]for y in range(3,8)]},
        {'name':'offcenter_goal','size':[15,12],'start':[2,3],'goal':[12,8], 'goal_world':[24.125,17.75]},
        {'name':'epoch_rollover','size':[12,12],'start':[2,2],'goal':[9,9],'initial_epoch':0x7ffffffe},
        {'name':'boundary_start','size':[12,12],'start':[0,2],'goal':[9,9]},
        {'name':'subunit_cells','size':[15,12],'start':[2,3],'goal':[12,8], 'shift':0,'cell_size':0.75},
    ]
    seed=0xc0deca11
    for trial in range(9):
        blocked=[]
        for y in range(1,12):
            for x in range(1,14):
                seed=(seed*1664525+1013904223)&0xffffffff
                if seed%5==0 and [x,y]not in [[2,3],[12,9]]:blocked.append([x,y])
        cases.append({'name':f'generated_obstacles_{trial}','size':[15,13],'start':[2,3],'goal':[12,9],'blocked':blocked})
    results=[]
    for precision,cw in [('Single24',0x007f),('Double53',0x027f),('Extended64',0x037f)]:
        for case in cases:
            result=SearchMachine(cw).run(case)
            results.append({'precision':precision,'input':case,'output':result})
            print(precision,case['name'],result['status'],len(result['popped']),result['finished'],flush=True)
    report={'schema':1,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),
            'producer_sha256':sha(Path(__file__).read_bytes()),
            'method':'Original search, concrete strategy, heap and neighbor filtering/cost code execute unchanged. Only final path materialization53D500 is intercepted to record endpoint.',
            'ranges':{name:{'start':hex(a),'end_exclusive':hex(b),'sha256':sha(original.range(a,b-a))}for name,(a,b)in RANGES.items()},
            'cases':results}
    (HERE/'native-search-fixtures.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps({'cases':len(results),'sha256':sha((HERE/'native-search-fixtures.json').read_bytes())}))

if __name__=='__main__':main()
