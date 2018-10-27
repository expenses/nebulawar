use specs::*;
use ships::StoredResource;
use components::*;

impl<'a> StorageGetter for WriteStorage<'a, Materials> {
    fn get(&self, entity: Entity) -> Option<&StoredResource> {
        self.get(entity).map(|storage| &storage.0)
    }

    fn get_mut(&mut self, entity: Entity) -> Option<&mut StoredResource> {
        self.get_mut(entity).map(|storage| &mut storage.0)
    }
}

impl<'a> StorageGetter for WriteStorage<'a, MineableMaterials> {
    fn get(&self, entity: Entity) -> Option<&StoredResource> {
        self.get(entity).map(|storage| &storage.0)
    }

    fn get_mut(&mut self, entity: Entity) -> Option<&mut StoredResource> {
        self.get_mut(entity).map(|storage| &mut storage.0)
    }
}

pub trait StorageGetter {
    fn get(&self, entity: Entity) -> Option<&StoredResource>;
    fn get_mut(&mut self, entity: Entity) -> Option<&mut StoredResource>;
}

pub fn transfer_between_same<G: StorageGetter>(getter: &mut G, entity_a: Entity, entity_b: Entity, amount: f32) -> Option<bool> {
    let can_transfer = {
        let storage_a = getter.get(entity_a)?;
        let storage_b = getter.get(entity_b)?;

        storage_a.transfer_amount(&storage_b, amount)
    };

    if can_transfer == 0.0 {
        Some(true)
    } else {
        getter.get_mut(entity_a)?.reduce(can_transfer);
        getter.get_mut(entity_b)?.increase(can_transfer);

        Some(false)
    }
} 
pub fn transfer_between_different<F: StorageGetter, T: StorageGetter>(from_getter: &mut F, to_getter: &mut T, from: Entity, to: Entity, amount: f32) -> Option<bool> {
    Some(from_getter.get_mut(from)?.transfer_to(to_getter.get_mut(to)?, amount) == 0.0)
}