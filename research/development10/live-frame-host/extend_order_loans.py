"""One-time source edit; retained solely as a diff reproduction aid."""
from pathlib import Path
p=Path('crates/aa-netcode/src/local_order_host.rs');s=p.read_text()
s=s.replace("pub struct World<'a, 'owner, E> {","""impl<'owner> AsRef<PlayersBorrow<'owner>> for PlayersBorrow<'owner>{fn as_ref(&self)->&Self{self}}
impl<'owner> AsMut<PlayersBorrow<'owner>> for PlayersBorrow<'owner>{fn as_mut(&mut self)->&mut Self{self}}
impl<'owner> AsRef<LocalClientBorrow<'owner>> for LocalClientBorrow<'owner>{fn as_ref(&self)->&Self{self}}
impl<'owner> AsMut<LocalClientBorrow<'owner>> for LocalClientBorrow<'owner>{fn as_mut(&mut self)->&mut Self{self}}

pub struct World<'a, 'owner, E, P=PlayersBorrow<'owner>, L=LocalClientBorrow<'owner>> {""")
s=s.replace("pub players: PlayersBorrow<'owner>,","pub players: P,").replace("pub local_client: LocalClientBorrow<'owner>,","pub local_client: L,")
s=s.replace("pub sync_a: &'owner RefCell<Synchronizer>,","pub sync_a: &'a RefCell<Synchronizer>,").replace("pub sync_s: &'owner mut Synchronizer,","pub sync_s: &'a mut Synchronizer,")
bounds="<'owner,E:Effects,P:AsRef<PlayersBorrow<'owner>>+AsMut<PlayersBorrow<'owner>>,L:AsRef<LocalClientBorrow<'owner>>+AsMut<LocalClientBorrow<'owner>>>"
s=s.replace("impl<E: Effects> World<'_, '_, E> {","impl"+bounds+" World<'_,'owner,E,P,L>{")
s=s.replace("struct PlayerContext<'a, 'world, 'owner, E> {","struct PlayerContext<'a, 'world, 'owner, E,P,L> {")
s=s.replace("world: &'a mut World<'world, 'owner, E>,","world: &'a mut World<'world, 'owner, E,P,L>,")
s=s.replace("impl<E: Effects> TryPlayerContext for PlayerContext<'_, '_, '_, E> {","impl"+bounds+" TryPlayerContext for PlayerContext<'_,'_,'owner,E,P,L>{")
s=s.replace("impl<E: Effects> OrderingHost for World<'_, '_, E> {","impl"+bounds+" OrderingHost for World<'_,'owner,E,P,L>{")
s=s.replace('self.players.', 'self.players.as_ref().').replace('self.world.players.', 'self.world.players.as_ref().')
s=s.replace('self.local_client.', 'self.local_client.as_ref().').replace('self.world.local_client.', 'self.world.local_client.as_ref().')
s=s.replace('&mut self.players,','self.players.as_mut(),').replace('&mut self.local_client,','self.local_client.as_mut(),').replace('&mut self.world.players','self.world.players.as_mut()')
s=s.replace('self.players\n','self.players.as_ref()\n').replace('self.world.players\n','self.world.players.as_ref()\n')
s=s.replace('self.world.local_client.as_ref().bind_player_74b350','self.world.local_client.as_mut().bind_player_74b350')
s=s.replace('self.players.as_ref()\n                    .set_seed_7710e0','self.players.as_mut()\n                    .set_seed_7710e0')
s+='\n///Short loans of the SAME player/client owners; shares every algorithm with World.\npub type BorrowedWorld<\'a,\'owner,\'loan,E> = World<\'a,\'owner,E,&\'loan mut PlayersBorrow<\'owner>,&\'loan mut LocalClientBorrow<\'owner>>;\n'
p.write_text(s)
