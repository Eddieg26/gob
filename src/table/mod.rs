use crate::dense::DenseSet;
use column::{Column, ColumnKey, ColumnType, Row, SelectedCell, SelectedRow};
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

pub struct TableLayout {
    columns: HashMap<ColumnKey, Column>,
}

impl TableLayout {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_type<C: ColumnType>(&mut self) -> &mut Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, Column::new::<C::Type>());
        self
    }

    pub fn with_type<C: ColumnType>(mut self) -> Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, Column::new::<C::Type>());
        self
    }

    pub fn add_field<C: 'static>(&mut self) -> &mut Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, Column::new::<C>());
        self
    }

    pub fn with_field<C: 'static>(mut self) -> Self {
        let key = ColumnKey::from::<C>();
        self.columns.insert(key, Column::new::<C>());
        self
    }

    pub fn add_column(&mut self, key: ColumnKey, column: Column) -> &mut Self {
        self.columns.insert(key, column);
        self
    }

    pub fn with_column(mut self, key: ColumnKey, column: Column) -> Self {
        self.columns.insert(key, column);
        self
    }

    pub fn build(self) -> Table {
        Table {
            columns: self.columns,
            rows: DenseSet::new(),
        }
    }
}

pub struct Table {
    columns: HashMap<ColumnKey, Column>,
    rows: DenseSet<RowIndex>,
}

impl Table {
    pub fn builder() -> TableLayout {
        TableLayout::new()
    }

    pub fn field<C: 'static>(&self, index: impl Into<RowIndex>) -> Option<&C> {
        let key = ColumnKey::from::<C>();
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(&key)?.get::<C>(index)
    }

    pub fn field_mut<C: 'static>(&self, index: impl Into<RowIndex>) -> Option<&mut C> {
        let key = ColumnKey::from::<C>();
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(&key)?.get_mut::<C>(index)
    }

    pub fn field_type<C: ColumnType>(&self, index: impl Into<RowIndex>) -> Option<&C::Type> {
        let key = ColumnKey::from::<C>();
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(&key)?.get::<C::Type>(index)
    }

    pub fn field_type_mut<C: ColumnType>(
        &self,
        index: impl Into<RowIndex>,
    ) -> Option<&mut C::Type> {
        let key = ColumnKey::from::<C>();
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(&key)?.get_mut::<C::Type>(index)
    }

    pub fn cell(&self, key: &ColumnKey, index: impl Into<RowIndex>) -> Option<SelectedCell> {
        let index = index.into();
        let index = self.rows.index(&index)?;
        self.columns.get(key)?.select(index)
    }

    pub fn select(&self, index: impl Into<RowIndex>) -> Option<SelectedRow> {
        let index = index.into();
        let index = self.rows.index(&index)?;
        let mut columns = HashMap::new();
        for (field, column) in &self.columns {
            columns.insert(field.clone(), column);
        }

        Some(SelectedRow::new(columns, index))
    }

    pub fn insert(&mut self, index: impl Into<RowIndex>, mut row: Row) {
        self.rows.insert(index.into());
        for (field, column) in &mut self.columns {
            let cell = row.remove_cell(field).unwrap();
            column.push_cell(cell);
        }
    }

    pub fn remove(&mut self, index: impl Into<RowIndex>) -> Option<Row> {
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
