use std::{
    any::TypeId,
    hash::{DefaultHasher, Hash, Hasher},
};

use blob::Blob;
use table::{column::Row, RowIndex, Table};

pub mod blob;
pub mod dense;
pub mod table;

fn main() {
    let mut blob = Blob::new::<u32>(3);
    blob.push(5);
    blob.push(10);
    blob.push(20);

    let mut ptr = blob.ptr::<u32>(1);
    *ptr += 5;

    let mut ptr2 = blob.ptr::<u32>(1);
    *ptr2 += 5;

    println!("{:?}", &ptr);

    // let mut row = Row::new();
    // row.add_field(ComponentId::from::<Player>(), Player::new(100))
    //     .add_field(ComponentId::from::<Name>(), Name::new("Player 1"));

    // let mut table = Table::builder()
    //     .with_type::<Player>(ComponentId::from::<Player>())
    //     .with_type::<Name>(ComponentId::from::<Name>())
    //     .build();

    // let index = RowIndex::new(0, 0);
    // table.insert(index, row);

    // let player = table.cell_mut::<Player>(index, &ComponentId::from::<Player>()).unwrap();
    // let player2 = table.cell_mut::<Player>(index, &ComponentId::from::<Player>()).unwrap();
    // player2.health = 200;
    // let name = table.cell::<Name>(index, &ComponentId::from::<Name>());
    // println!("{:?} {:?}", player, player2);
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ComponentId(u64);

impl ComponentId {
    pub fn from<C: Component>() -> Self {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<C>().hash(&mut hasher);

        ComponentId(hasher.finish())
    }
}

#[derive(Debug, Clone)]
struct Name(String);

impl Name {
    pub fn new(name: &str) -> Self {
        Name(name.to_string())
    }
}

impl Component for Name {}

#[derive(Debug)]
struct Player {
    health: u32,
}

impl Player {
    pub fn new(health: u32) -> Player {
        Player { health }
    }
}

impl Component for Player {}

pub trait Component: 'static {}

// impl<C: Component> ColumnType<C> for C {
//     type Type = Self;
// }
