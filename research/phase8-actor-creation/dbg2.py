import sys
from native_object_definition import cases, run
from unicorn.x86_const import *
import native_object_definition as nod
label, c = cases()[0]
from actor_machine import ActorMachine
orig = ActorMachine._hook
last = []
def hook(self, u, pc, size, user):
    last.append(pc)
    if len(last) > 30: last.pop(0)
    return orig(self, u, pc, size, user)
ActorMachine._hook = hook
try:
    print(run(c))
except Exception as e:
    print('ERR', e, [hex(x) for x in last])
