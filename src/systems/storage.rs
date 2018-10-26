use specs::*;
use ships::StoredResource;
use common_components::*;

impl<'a> StorageGetter for WriteStorage<'a, Fuel> {
    fn get(&self, entity: Entity) -> Option<&StoredResource> {
        self.get(entity).map(|storage| &storage.0)
    }

    fn get_mut(&mut self, entity: Entity) -> Option<&mut StoredResource> {
        self.get_mut(entity).map(|storage| &mut storage.0)
    }
}

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

pub fn transfer_from_storages<G: StorageGetter>(fuel: &mut G, ship_a: Entity, ship_b: Entity, amount: f32) -> Option<bool> {
    let can_transfer = {
        let fuel_a = fuel.get(ship_a)?;
        let fuel_b = fuel.get(ship_b)?;

        fuel_a.transfer_amount(&fuel_b, amount)
    };

    if can_transfer == 0.0 {
        Some(true)
    } else {
        fuel.get_mut(ship_a)?.reduce(can_transfer);
        fuel.get_mut(ship_b)?.increase(can_transfer);

        Some(false)
    }
} 
pub fn transfer_between_different<F: StorageGetter, T: StorageGetter>(from_getter: &mut F, to_getter: &mut T, from: Entity, to: Entity, amount: f32) -> Option<bool> {
    Some(from_getter.get_mut(from)?.transfer_to(to_getter.get_mut(to)?, amount) == 0.0)
}