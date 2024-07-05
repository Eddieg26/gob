use crate::blob::Blob;
use std::{
    any::TypeId,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

pub struct ColumnCell {
    data: Blob,
}

impl ColumnCell {
    pub fn from<T: 'static>(value: T) -> Self {
        let mut data = Blob::new::<T>(1);
        data.push(value);

        Self { data }
    }

    pub fn value<T: 'static>(&self) -> Option<&T> {
        self.data.get::<T>(0)
    }

    pub fn value_mut<T: 'static>(&self) -> Option<&mut T> {
        self.data.get_mut::<T>(0)
    }

    pub fn take<T: 'static>(mut self) -> T {
        self.data.remove(0)
    }
}

pub struct SelectedCell<'a> {
    column: &'a Column,
    index: usize,
}

impl<'a> SelectedCell<'a> {
    fn new(column: &'a Column, index: usize) -> Self {
        Self { column, index }
    }

    pub fn value<T: 'static>(&self) -> Option<&T> {
        self.column.get::<T>(self.index)
    }

    pub fn value_mut<T: 'static>(&self) -> Option<&mut T> {
        self.column.get_mut::<T>(self.index)
    }
}

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<T: 'static>() -> Self {
        Self {
            data: Blob::new::<T>(0),
        }
    }

    pub fn copy(column: &Column) -> Self {
        Column {
            data: Blob::with_layout(column.data.layout().clone(), 0, column.data.drop().copied()),
        }
    }

    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        self.data.get::<T>(index)
    }

    pub fn get_mut<T: 'static>(&self, index: usize) -> Option<&mut T> {
        self.data.get_mut::<T>(index)
    }

    pub fn push<T: 'static>(&mut self, value: T) {
        self.data.push(value)
    }

    pub fn insert<T: 'static>(&mut self, index: usize, value: T) {
        self.data.insert(index, value)
    }

    pub fn extend(&mut self, column: Column) {
        self.data.extend(column.data)
    }

    pub fn remove<T: 'static>(&mut self, index: usize) -> T {
        self.data.remove(index)
    }

    pub fn swap_remove<T: 'static>(&mut self, index: usize) -> T {
        self.data.swap_remove(index)
    }

    pub fn select(&self, index: usize) -> Option<SelectedCell> {
        if index >= self.len() {
            None
        } else {
            Some(SelectedCell::new(self, index))
        }
    }

    pub fn push_cell(&mut self, cell: ColumnCell) {
        self.data.extend(cell.data)
    }

    pub fn insert_cell(&mut self, index: usize, cell: ColumnCell) {
        self.data.insert_blob(index, cell.data)
    }

    pub fn remove_data(&mut self, index: usize) -> ColumnCell {
        let data = self.data.remove_blob(index);
        ColumnCell { data }
    }

    pub fn swap_remove_data(&mut self, index: usize) -> ColumnCell {
        let data = self.data.swap_remove_blob(index);
        ColumnCell { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn clear(&mut self) {
        self.data.clear()
    }
}

impl From<ColumnCell> for Column {
    fn from(cell: ColumnCell) -> Self {
        Column { data: cell.data }
    }
}

pub trait ColumnType: 'static {
    type Type;

    fn name() -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnKey(u64);

impl ColumnKey {
    pub fn from<K: 'static>() -> Self {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<K>().hash(&mut hasher);

        ColumnKey(hasher.finish())
    }
}

pub struct Row {
    columns: HashMap<ColumnKey, ColumnCell>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_type<C: ColumnType>(&mut self, value: C::Type) -> &mut Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, ColumnCell::from(value));
        self
    }

    pub fn remove_type<C: ColumnType>(&mut self) -> Option<C::Type> {
        let key = ColumnKey::from::<C>();
        self.columns.remove(&key)?.take()
    }

    pub fn add_field<C: 'static>(&mut self, value: C) -> &mut Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, ColumnCell::from(value));
        self
    }

    pub fn remove_field<C: 'static>(&mut self) -> Option<C> {
        let key = ColumnKey::from::<C>();
        self.columns.remove(&key)?.take()
    }

    pub fn add_cell(&mut self, key: ColumnKey, cell: ColumnCell) -> &mut Self {
        self.columns.insert(key, cell.into());
        self
    }

    pub fn remove_cell(&mut self, key: &ColumnKey) -> Option<ColumnCell> {
        self.columns.remove(key)
    }

    pub fn field<C: 'static>(&self) -> Option<&C> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.value::<C>()
    }

    pub fn field_mut<C: 'static>(&mut self) -> Option<&mut C> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.value_mut::<C>()
    }

    pub fn fields(&self) -> std::collections::hash_map::Keys<ColumnKey, ColumnCell> {
        self.columns.keys()
    }

    pub fn field_type<C: ColumnType>(&self) -> Option<&C::Type> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.value::<C::Type>()
    }

    pub fn field_type_mut<C: ColumnType>(&self) -> Option<&mut C::Type> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.value_mut::<C::Type>()
    }

    pub fn cell(&self, key: &ColumnKey) -> Option<&ColumnCell> {
        self.columns.get(key)
    }
}

pub struct SelectedRow<'a> {
    columns: HashMap<ColumnKey, &'a Column>,
    index: usize,
}

impl<'a> SelectedRow<'a> {
    pub fn new(columns: HashMap<ColumnKey, &'a Column>, index: usize) -> Self {
        Self { columns, index }
    }

    pub fn field<C: 'static>(&self) -> Option<&C> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.get::<C>(self.index)
    }

    pub fn field_mut<C: 'static>(&self) -> Option<&mut C> {
        let key = ColumnKey::from::<C>();
        self.columns.get(&key)?.get_mut::<C>(self.index)
    }

    pub fn fields(&self) -> std::collections::hash_map::Keys<ColumnKey, &'a Column> {
        self.columns.keys()
    }
}
