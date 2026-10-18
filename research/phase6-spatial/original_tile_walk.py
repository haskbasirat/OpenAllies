"""Original TileWalk initialization, next-candidate and in-grid next-cell order.
Only native diagnostic logging is intercepted; every arithmetic/branch executes.
"""
from pathlib import Path
import hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine, original, modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def signed(x):return x if x<0x80000000 else x-0x100000000
def sha(b):return hashlib.sha256(b).hexdigest()

class WalkMachine(Machine):
    MAP=0x2003000;OUTPUT=0x2004000
    def __init__(self,precision):
        super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[precision])
        self.visited={};self.diagnostics=0;self.u.hook_add(UC_HOOK_CODE,self.hook)
    def hook(self,u,pc,size,_):
        if pc==0x404f60:
            self.diagnostics+=1;sp=u.reg_read(UC_X86_REG_ESP)
            u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
        elif original.base<=pc<original.base+original.image_size:self.visited[pc]=size
    def state(self):
        return {'current':[signed(self.r(self.STATE+8)),signed(self.r(self.STATE+12))],
            'end':[signed(self.r(self.STATE+16)),signed(self.r(self.STATE+20))],
            'delta_bits':[self.r(self.STATE+24),self.r(self.STATE+28)],
            'point_bits':[self.r(self.STATE+32),self.r(self.STATE+36)],
            'started':bool(self.u.mem_read(self.STATE+40,1)[0]),'remaining':signed(self.r(self.STATE+44))}
    def initialize(self,c):
        self.reset(0);g=c['grid'];self.w(self.STATE+4,self.MAP)
        self.w(self.MAP+4,g['coordinate_shift']);self.w(self.MAP+8,g['cell_size_bits'])
        self.w(self.MAP+28,g['width']);self.w(self.MAP+32,g['height']);self.diagnostics=0
        for i,v in enumerate(c['start_bits']+c['end_bits']):self.w(self.STACK+4+i*4,v)
        self.u.emu_start(0x4dd030,self.STOP,count=100000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+20
        return self.state()
    def next(self,entry):
        self.w(self.STACK,self.STOP);self.w(self.STACK+4,self.OUTPUT)
        self.u.reg_write(UC_X86_REG_ESP,self.STACK);self.u.reg_write(UC_X86_REG_ECX,self.STATE)
        self.u.emu_start(entry,self.STOP,count=100000)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
        success=bool(self.u.reg_read(UC_X86_REG_EAX)&255)
        return {'cell':[signed(self.r(self.OUTPUT)),signed(self.r(self.OUTPUT+4))] if success else None,'state':self.state()}

def cases():
    pairs=[([0,0],[0,0]),([0,0],[7,7]),([7,7],[0,0]),([1,1],[9,9]),([9,1],[1,9]),
        ([1,9],[9,1]),([9,9],[1,1]),([1,1],[9,1]),([1,1],[1,9]),([9,1],[1,1]),
        ([-3,1],[19,1]),([3,-5],[3,15]),([-4,-4],[20,20]),([1.99999,1],[2.00001,7]),
        ([1,1.99999],[7,2.00001]),([1,1],[1.0001,9]),([1,1],[9,1.0001]),
        ([-0.0,-0.0],[15.99999,9.99999]),([0,1],[16,1]),([0,0],[16,10])]
    rng=random.Random(0x4dd150)
    pairs += [([rng.uniform(-4,20),rng.uniform(-4,14)],[rng.uniform(-4,20),rng.uniform(-4,14)])for _ in range(100)]
    rows=[]
    for precision in ['Single24','Double53','Extended64']:
        for index,(start,end) in enumerate(pairs):
            rows.append({'name':f'walk_{index}_{precision}','precision':precision,
                'grid':{'width':8,'height':5,'coordinate_shift':1,'cell_size_bits':bits(2)},
                'start_bits':list(map(bits,start)),'end_bits':list(map(bits,end))})
    return rows
def main():
    rows=[];visited={}
    for c in cases():
        m=WalkMachine(c['precision']);initial=m.initialize(c);raw=[]
        for _ in range(initial['remaining']+2):
            row=m.next(0x4dd150);raw.append(row)
            if row['cell'] is None:break
        else:raise AssertionError('native raw iterator did not finish')
        m.initialize(c);filtered=[]
        for _ in range(initial['remaining']+2):
            row=m.next(0x4ddde0);filtered.append(row)
            if row['cell'] is None:break
        else:raise AssertionError('native bounded iterator did not finish')
        rows.append({'input':c,'initial':initial,'raw':raw,'filtered':filtered,'diagnostics':m.diagnostics})
        visited.update(m.visited)
    ranges=[]
    for pc,size in sorted(visited.items()):
        if ranges and ranges[-1][1]==pc:ranges[-1][1]=pc+size
        else:ranges.append([pc,pc+size])
    pins=[]
    for start,end in ranges:
        b=original.range(start,end-start);assert b==modified.range(start,end-start)
        pins.append({'start':hex(start),'end':hex(end),'sha256':sha(b)})
    result={'schema':1,'passed':True,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'fixtures':rows,'executed_ranges':pins}
    path=OUT/'native-tile-walk.json';path.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
