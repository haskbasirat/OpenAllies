///Synchronous disjoint zone loan. Every actor read resolves its SAME fields;
///the current runtime allocation is never detached or replaced by an empty one.
pub(crate) struct ZoneFields<'a> {
    pub owner: ActorId,
    pub key: ObjectKey,
    pub definition: &'a RuntimeDefinition,
    pub kind: LoadKind,
    pub thing: &'a ThingCore,
    pub object: &'a mut ObjectFields,
    pub components: &'a mut Components<RuntimeComponent>,
    pub words: &'a BTreeMap<u32, u32>,
    placement: &'a PlacementOwner,
    source: &'a Source,
    identities: &'a IdentityRegistry<ObjectKey>,
    others: BTreeMap<ObjectKey, &'a mut Entity>,
    zone_index: usize,
    zone_key: u32,
    zone_before: &'a [Option<aa_zones::zone::Zone>],
    zone_after: &'a [Option<aa_zones::zone::Zone>],
}
impl ZoneFields<'_> {
    fn prior(&self, id: ActorId) -> Result<&Entity> {
        let key = self
            .identities
            .get(id)
            .ok_or(Error::Missing("zone current Actor generation"))?
            .value;
        self.others
            .get(&key)
            .map(|v| &**v)
            .ok_or(Error::Missing("zone actual other Actor allocation"))
    }
    pub fn present(&self, id: ActorId) -> bool {
        if id == self.owner {
            self.identities.get(id).is_some_and(|v| v.value == self.key)
        } else {
            self.prior(id).is_ok()
        }
    }
    pub fn definition(&self, id: ActorId) -> Result<&RuntimeDefinition> {
        if id == self.owner {
            return Ok(self.definition);
        }
        Ok(&self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .definition)
    }
    pub fn source(&self, id: ActorId) -> Result<&Source> {
        if id == self.owner {
            Ok(self.source)
        } else {
            Ok(&self.prior(id)?.source)
        }
    }
    pub fn same_definition_proof(&self, id: ActorId) -> Result<bool> {
        if id == self.owner {
            return Ok(crate::load_spatial::same_proof(
                &self.thing.definition,
                &self.definition.proof,
            ));
        }
        let runtime = self.prior(id)?.runtime.as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?;
        Ok(crate::load_spatial::same_proof(
            &runtime.thing.definition,
            &runtime.definition.proof,
        ))
    }
    pub fn components(&self, id: ActorId) -> Result<&Components<RuntimeComponent>> {
        if id == self.owner {
            return Ok(self.components);
        }
        self.prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .components()
    }
    pub fn words(&self, id: ActorId) -> Result<&BTreeMap<u32, u32>> {
        if id == self.owner {
            return Ok(self.words);
        }
        Ok(&self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .actor
            .as_ref()
            .ok_or(Error::Missing("zone actual Actor storage"))?
            .words)
    }
    pub fn kind(&self, id: ActorId) -> Result<(LoadKind, bool, bool)> {
        if id == self.owner {
            return Ok((self.kind, true, true));
        }
        let r = self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?;
        Ok((r.kind, r.object.is_some(), r.actor.is_some()))
    }
    pub fn placement(&self, id: ActorId, spatial: &impl Spatial) -> Result<PlacementState> {
        let p = if id == self.owner {
            self.placement
        } else {
            &self
                .prior(id)?
                .runtime
                .as_ref()
                .ok_or(Error::Missing("zone actual Object storage"))?
                .placement
        };
        match p {
            PlacementOwner::Staged(p) => Ok(*p),
            PlacementOwner::Spatial => spatial.placement(id),
        }
    }
    pub fn zone_key(&self, id: ActorId, index: usize) -> Result<u32> {
        if id == self.owner {
            return Ok(if index == self.zone_index {
                self.zone_key
            } else if index < self.zone_index {
                self.zone_before
                    .get(index)
                    .and_then(Option::as_ref)
                    .map_or(0, |z| z.key)
            } else {
                self.zone_after
                    .get(index - self.zone_index - 1)
                    .and_then(Option::as_ref)
                    .map_or(0, |z| z.key)
            });
        }
        Ok(self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .actor
            .as_ref()
            .ok_or(Error::Missing("zone actual Actor storage"))?
            .zones_148
            .zones()
            .slots
            .get(index)
            .and_then(Option::as_ref)
            .map_or(0, |z| z.key))
    }
}
impl ActorStore {
    pub(crate) fn with_zone_fields<R>(
        &mut self,
        key: ObjectKey,
        index: usize,
        call: impl FnOnce(&mut ZoneFields<'_>, &mut aa_zones::zone::Zone) -> R,
    ) -> Result<R> {
        let owner = self
            .id(key)
            .ok_or(Error::Missing("zone current Actor generation"))?;
        let mut current = None;
        let mut others = BTreeMap::new();
        for (k, e) in self.entities.iter_mut() {
            if *k == key {
                current = Some(&mut **e)
            } else {
                others.insert(*k, &mut **e);
            }
        }
        let entity = current.ok_or(Error::Missing("zone actual current Entity"))?;
        let r = entity
            .runtime
            .as_mut()
            .ok_or(Error::Missing("zone actual current runtime"))?;
        let actor = r
            .actor
            .as_mut()
            .ok_or(Error::Missing("zone actual current Actor"))?;
        let slots = &mut actor.zones_148.zones_mut().slots;
        if index >= slots.len() {
            return Err(Error::Missing("zone actual current slot"));
        }
        let (before, tail) = slots.split_at_mut(index);
        let (slot, after) = tail.split_first_mut().expect("checked current slot");
        let zone = slot
            .as_mut()
            .ok_or(Error::Missing("zone actual current allocation"))?;
        let mut fields = ZoneFields {
            owner,
            key,
            definition: &r.definition,
            kind: r.kind,
            thing: &r.thing,
            object: r
                .object
                .as_mut()
                .ok_or(Error::Missing("zone actual current Object"))?,
            components: &mut actor.components,
            words: &actor.words,
            placement: &r.placement,
            source: &entity.source,
            identities: &self.identities,
            others,
            zone_index: index,
            zone_key: zone.key,
            zone_before: before,
            zone_after: after,
        };
        Ok(call(&mut fields, zone))
    }
}
