// column_pos.rs

use crate::chunk;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ColumnPos {
    pub x: i32,
    pub z: i32,
}

impl ColumnPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self {
            x,
            z,
        }
    }

    pub fn block_x(&self) -> i64 {
        self.x as i64 * chunk::SIZE as i64
    }

    pub fn block_z(&self) -> i64 {
        self.z as i64 * chunk::SIZE as i64
    }

    pub fn neighbor(&self, dx: i32, dz: i32) -> ColumnPos {
        ColumnPos::new(
            self.x + dx,
            self.z + dz
        )
    }
}
