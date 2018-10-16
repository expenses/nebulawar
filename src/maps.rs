use std::ops::*;
use std::collections::*;
use std::collections::hash_map::*;
use std::hash::*;
use std::iter::*;

pub trait ID: Hash + Copy + Eq + Default {
    fn increment(&mut self);
}

pub trait IDed<I> {
    fn set_id(&mut self, id: I);
}

#[derive(Deserialize, Serialize)]
pub struct AutoIDMap<I: ID, T> {
    next_id: I,
    inner: HashMap<I, T>
}

impl<I: ID, T: IDed<I>> AutoIDMap<I, T> {
    pub fn new() -> Self {
        Self {
            next_id: I::default(),
            inner: HashMap::new()
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            next_id: I::default(),
            inner: HashMap::with_capacity(cap)
        }
    }

    pub fn push(&mut self, mut item: T) -> I {
        let id = self.next_id;

        item.set_id(id);
        self.inner.insert(id, item);
        self.next_id.increment();

        id
    }

    pub fn get(&self, id: I) -> Option<&T> {
        self.inner.get(&id)
    }

    pub fn get_mut(&mut self, id: I) -> Option<&mut T> {
        self.inner.get_mut(&id)
    }

    pub fn iter(&self) -> Values<I, T> {
        self.inner.values()
    }

    pub fn iter_mut(&mut self) -> ValuesMut<I, T> {
        self.inner.values_mut()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<I: ID, T> Index<I> for AutoIDMap<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &T {
        &self.inner[&index]
    }
}

impl<I: ID, T: IDed<I>> IndexMut<I> for AutoIDMap<I, T> {
    fn index_mut(&mut self, index: I) -> &mut T {
        self.get_mut(index).unwrap()
    }
}

impl<I: ID, T: IDed<I>> FromIterator<T> for AutoIDMap<I, T> {
    fn from_iter<IT: IntoIterator<Item=T>>(iter: IT) -> Self {
        let iter = iter.into_iter();
        let (min, max) = iter.size_hint();
        let size = max.unwrap_or(min);

        let mut map = Self::with_capacity(size);

        for item in iter {
            map.push(item);
        }

        map
    }
}