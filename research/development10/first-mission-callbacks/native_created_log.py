"""Actual4FBA50 success-log argument reads after named fresh creation seams.

The successful4FA590/4F9C10 creation answers are laboratory inputs.404F90 is
a captured diagnostic transport seam: no threshold, formatter or native sink
parity is claimed. Original/v14 run separately at all3 PCs. The creation seam
changes current Params IDS/XY, proving the logger does not use an entry copy.
"""
from pathlib import Path
import hashlib, json, struct, sys

HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-map-replacement-drain'))
from native_replacement_drain import Replacement,run,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ

class ReadStop(Exception): pass
class Arguments(Replacement):
    def __init__(self,image):
        self.after_create=False
        self.selected_input=None
        self.argument_reads=[]
        super().__init__(image)
        self.u.hook_add(UC_HOOK_MEM_READ,self.arguments_read)

    def arguments_read(self,u,access,address,size,value,user):
        if not self.after_create: return
        watched={self.params+0xc:'y',self.params+4:'definition',self.log_definition+8:'ids',self.params+8:'x'}
        if address==self.params+0x10:
            raise AssertionError('success diagnostic must not read Z')
        if address in watched:
            name=watched[address]
            self.argument_reads.append(name)
            if self.selected_input.get('stop')==name: raise ReadStop(name)

    def hook(self,u,pc,size,user):
        if self.replacement_ready and pc in (0x4f9c10,0x4fa590):
            super().hook(u,pc,size,user)
            self.log_definition=self.alloc(0x20)
            self.cs(self.log_definition+8,self.selected_input['ids'])
            self.w(self.params+4,self.log_definition)
            self.w(self.params+8,self.selected_input['x'])
            self.w(self.params+0xc,self.selected_input['y'])
            self.w(self.params+0x10,0x7f800001)
            self.after_create=True
            return
        if self.replacement_ready and pc==0x404f90:
            self.arguments=dict(logger=hex(self.arg()),file=self.text(self.arg(1)),
                line=self.arg(2),caller=self.text(self.arg(3)),format=self.text(self.arg(4)),
                ids=self.text(self.arg(5)),x_double=f'{self.arg(7):08x}{self.arg(6):08x}',
                y_double=f'{self.arg(9):08x}{self.arg(8):08x}')
            self.event('created_log')
            self.ret();return
        super().hook(u,pc,size,user)

def one(m,pc,c):
    m.after_create=False;m.selected_input=c;m.argument_reads=[];m.arguments=None
    try:
        output,pins=run(m,pc,dict(source=0,properties=[],actor=c['actor']))
        result=dict(returned=output['returned'],error=None,reads=m.argument_reads,args=m.arguments)
    except ReadStop as error:
        pins=m.actual.copy()
        result=dict(returned=None,error=str(error),reads=m.argument_reads,args=m.arguments)
    if not c.get('stop'):
        assert result['reads']==['y','definition','ids','x'],result
        expected=lambda b:struct.pack('>d',struct.unpack('<f',struct.pack('<I',b))[0]).hex()
        assert result['args']['x_double']==expected(c['x'])
        assert result['args']['y_double']==expected(c['y'])
        assert result['args']['ids']==c['ids']
        assert result['args']['line']==0x135e
        assert result['args']['format']=='created replacement %s at %f,%f'
        assert result['args']['caller']=='KKC_PlaceholderResolver::CreateReplacement'
        assert result['returned']==1
    else:
        assert result['args'] is None and result['reads'][-1]==c['stop']
    return result,pins

def main():
    pairs=[(0,0),(0x80000000,0),(0x3f800000,0xc0000000),(1,0x80000001),
           (0x7f7fffff,0xff7fffff),(1109393408,1129840640),
           (0x3f000001,0x3effffff),(0x4b800001,0xcb800001)]
    cases=[dict(actor=a,x=x,y=y,ids='current_\u03a9_target')
           for a in [False,True] for x,y in pairs]
    cases += [dict(actor=True,x=0x3f800000,y=0xc0000000,ids='current',stop=s)
              for s in ['y','definition','ids','x']]
    rows=[];pins={}
    for image in ['original','v14']:
        m=Arguments(image)
        for pc in PC:
            for case in cases:
                output,visited=one(m,pc,case);pins.update(visited)
                rows.append(dict(image=image,precision=pc,input=case,output=output))
    canonical=lambda image:[{k:v for k,v in row.items() if k!='image'} for row in rows if row['image']==image]
    assert canonical('original')==canonical('v14')
    instructions=[]
    for address,size in sorted(pins.items()):
        if PE0.base<=address<PE0.base+PE0.image_size:
            raw=PE0.range(address,size);assert raw==PE1.range(address,size)
            instructions.append(dict(address=hex(address),bytes=raw.hex()))
    doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             original_sha256=hashlib.sha256(PE0.b).hexdigest(),v14_sha256=hashlib.sha256(PE1.b).hexdigest(),
             cases=rows,instructions=instructions,seams=['fresh creation4F9C10/4FA590','404F90 capture-only diagnostic transport','inherited CString allocation/storage'])
    path=HERE/'native-created-log.json'
    if '--create' in sys.argv:
        with path.open('x') as output: json.dump(doc,output,indent=2);output.write('\n')
    else: assert json.loads(path.read_text())==doc
    print(json.dumps(dict(paired_rows=len(rows)//2,executions=len(rows),pins=len(instructions),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__': main()
