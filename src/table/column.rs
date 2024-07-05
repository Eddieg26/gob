use crate::blob::Blob;
use std::{collections::HashMap, hash::Hash};

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

pub struct Row<K: Clone + Hash + Eq> {
    columns: HashMap<K, ColumnCell>,
}

impl<K: Clone + Hash + Eq> Row<K> {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_field<C: 'static>(&mut self, field: K, value: C) -> &mut Self {
        self.columns.insert(field, ColumnCell::from(value));
        self
    }

    pub fn remove_field<C: 'static>(&mut self, field: &K) -> Option<C> {
        self.columns.remove(field)?.take()
    }

    pub fn add_cell(&mut self, field: K, cell: ColumnCell) -> &mut Self {
        self.columns.insert(field, cell.into());
        self
    }

    pub fn remove_cell(&mut self, field: &K) -> Option<ColumnCell> {
        self.columns.remove(field)
    }

    pub fn field<C: 'static>(&self, field: &K) -> Option<&C> {
        self.columns.get(field)?.value::<C>()
    }

    pub fn field_mut<C: 'static>(&mut self, field: &K) -> Option<&mut C> {
        self.columns.get(field)?.value_mut::<C>()
    }

    pub fn fields(&self) -> std::collections::hash_map::Keys<K, ColumnCell> {
        self.columns.keys()
    }
}

pub struct SelectedRow<'a, K: Hash + Eq> {
    columns: HashMap<K, &'a Column>,
    index: usize,
}

impl<'a, K: Hash + Eq> SelectedRow<'a, K> {
    pub fn new(columns: HashMap<K, &'a Column>, index: usize) -> Self {
        Self { columns, index }
    }

    pub fn field<C: 'static>(&self, field: &K) -> Option<&C> {
        self.columns.get(field)?.get::<C>(self.index)
    }

    pub fn field_mut<C: 'static>(&self, field: &K) -> Option<&mut C> {
        self.columns.get(field)?.get_mut::<C>(self.index)
    }

    pub fn fields(&self) -> std::collections::hash_map::Keys<K, &'a Column> {
        self.columns.keys()
    }
}
