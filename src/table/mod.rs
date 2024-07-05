use crate::dense::DenseSet;
use column::{Column, Row, SelectedRow};
use std::{collections::HashMap, hash::Hash};

pub mod column;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct RowIndex {
    id: usize,
    gen: usize,
}

impl RowIndex {
    pub fn new(id: usize, gen: usize) -> Self {
        Self { id, gen }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn gen(&self) -> usize {
        self.gen
    }
}

pub struct TableLayout<K: Clone + Hash + Eq> {
    columns: HashMap<K, Column>,
}

impl<K: Clone + Hash + Eq> TableLayout<K> {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_type<C: 'static>(&mut self, key: K) -> &mut Self {
        self.columns.insert(key, Column::new::<C>());
        self
    }

    pub fn with_type<C: 'static>(mut self, key: K) -> Self {
        self.columns.insert(key, Column::new::<C>());
        self
    }

    pub fn add_column(&mut self, key: K, column: Column) -> &mut Self {
        self.columns.insert(key, column);
        self
    }

    pub fn with_column(mut self, key: K, column: Column) -> Self {
        self.columns.insert(key, column);
        self
    }

    pub fn build(self) -> Table<K> {
        Table {
            columns: self.columns,
            rows: DenseSet::new(),
        }
    }
}

pub struct Table<F: Clone + Hash + Eq> {
    columns: HashMap<F, Column>,
    rows: DenseSet<RowIndex>,
}

impl<F: Clone + Hash + Eq> Table<F> {
    pub fn builder() -> TableLayout<F> {
        TableLayout::new()
    }

    pub fn cell<C: 'static>(&self, index: impl Into<RowIndex>, field: &F) -> Option<&C> {
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(field)?.get::<C>(index)
    }

    pub fn cell_mut<C: 'static>(&self, index: impl Into<RowIndex>, field: &F) -> Option<&mut C> {
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(field)?.get_mut::<C>(index)
    }

    pub fn select(&self, index: impl Into<RowIndex>) -> Option<SelectedRow<F>> {
        let index = index.into();
        let index = self.rows.index(&index)?;
        let mut columns = HashMap::new();
        for (field, column) in &self.columns {
            columns.insert(field.clone(), column);
        }

        Some(SelectedRow::new(columns, index))
    }

    pub fn insert(&mut self, index: impl Into<RowIndex>, mut row: Row<F>) {
        self.rows.insert(index.into());
        for (field, column) in &mut self.columns {
            let cell = row.remove_cell(field).unwrap();
            column.push_cell(cell);
        }
    }

    pub fn remove(&mut self, index: impl Into<RowIndex>) -> Option<Row<F>> {
        let index = index.into();
        let idx = self.rows.index(&index)?;
        self.rows.remove(&index)?;
        let mut row = Row::new();
        for (field, column) in &mut self.columns {
            let cell = column.swap_remove_data(idx);
            row.add_cell(field.clone(), cell);
        }

        Some(row)
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        for column in self.columns.values_mut() {
            column.clear();
        }
    }
}
