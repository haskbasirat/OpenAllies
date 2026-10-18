"""76F600 actual lazy guard, stop before any successful76BA90 child.
Default immutable original/v14/allPC replay. Does not prove positive UI path.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_local_heartbeat import Machine as Base
from native_heartbeat import env,node,PC
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP
def main():
    rows=[];pins={}
    for image in ('original','v14'):
        m=Base(image)
        def stop():m.stopped='human_opposition';m.u.emu_stop()
        m.stubs[0x76ba90]=stop
        for p in PC:
            for enabled in (0,1,128):
                for later in (0,255):
                    m.setup({'env':env(),'node':node(),'conns':[],'list':['node']});m.fpu(p)
                    m.b(m.session+0x16d,enabled);m.b(m.session+0x16e,later);m.stopped=None;m.active_capture=True
                    m.w(m.stack,m.stop);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.emu_start(0x76f600,m.stop,count=10000)
                    if m.stopped is None:assert m.u.reg_read(UC_X86_REG_EIP)==m.stop
                    m.active_capture=False
                    assert m.rb(m.session+0x16e)==later
                    rows.append({'image':image,'precision':p,'enabled':enabled,'later':later,'stopped':m.stopped is not None})
        pins.update(m.executed)
    data=json.dumps({'scope':'76F600 guard and required76BA90 boundary only, no later Session16E/World/UI effects','rows':rows,'executed_pins':pins},sort_keys=True,indent=2)+'\n'
    path=HERE/'native-frame-end-condition.json'
    if '--write' in sys.argv:path.write_text(data,encoding='utf-8',newline='\n')
    else:assert path.read_text(encoding='utf-8')==data
    print(f'{len(rows)} rows / {len(pins)} genuine executed pins PASS')
if __name__=='__main__':main()
