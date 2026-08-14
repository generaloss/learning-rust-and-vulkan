// level.rs

use std::collections::hash_map::{Iter, IterMut};
use std::collections::HashMap;
use crate::chunk::Chunk;
use crate::chunk_column::ChunkColumn;
use crate::column_pos::ColumnPos;

pub struct Level {
    columns: HashMap<ColumnPos, ChunkColumn>,
}

impl Level {

    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn get_column(&self, position: &ColumnPos) -> Option<&ChunkColumn> {
        self.columns.get(position)
    }

    pub fn get_column_mut(&mut self, position: &ColumnPos) -> Option<&mut ChunkColumn> {
        self.columns.get_mut(position)
    }

    pub fn get_chunk(&self, x: i32, y: i32, z: i32) -> Option<&Chunk> {
        if let Some(column) = self.get_column(&ColumnPos::new(x, z)) {
            column.get_chunk(y)
        } else {
            None
        }
    }

    pub fn get_chunk_mut(&mut self, x: i32, y: i32, z: i32) -> Option<&mut Chunk> {
        if let Some(column) = self.get_column_mut(&ColumnPos::new(x, z)) {
            column.get_chunk_mut(y)
        } else {
            None
        }
    }

    pub fn put_chunk(&mut self, chunk: Chunk) {
        if let Some(column) = self.get_column_mut(&ColumnPos::new(chunk.pos.x, chunk.pos.z)) {
            column.put_chunk(chunk)
        } else {
            let pos = ColumnPos::new(chunk.pos.x, chunk.pos.z);
            let mut column = ChunkColumn::new(pos);
            column.put_chunk(chunk);
            self.columns.insert(pos, column);
        }
    }

    pub fn remove_chunk_at(&mut self, x: i32, y: i32, z: i32) -> Option<Chunk> {
        let column_pos = &ColumnPos::new(x, z);

        if let Some(column) = self.columns.get_mut(column_pos) {
            let chunk = column.remove_chunk_at(y);

            if column.is_empty() {
                self.columns.remove(column_pos);
            }

            chunk
        } else {
            None
        }
    }

    pub fn remove_chunk(&mut self, chunk: &Chunk) -> Option<Chunk> {
        self.remove_chunk_at(chunk.pos.x, chunk.pos.y, chunk.pos.z)
    }
    
    pub fn iter(&self) -> Iter<ColumnPos, ChunkColumn> {
        self.columns.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<ColumnPos, ChunkColumn> {
        self.columns.iter_mut()
    }

    pub fn for_each_chunk<F: FnMut(&Chunk)>(&self, mut iter: F) {
        for (_pos, column) in self.columns.iter() {
            for chunk in column.iter() {
                iter(chunk);
            }
        }
    }

    pub fn for_each_chunk_mut<F: FnMut(&mut Chunk)>(&mut self, mut iter: F) {
        for (_pos, column) in self.columns.iter_mut() {
            for chunk in column.iter_mut() {
                iter(chunk);
            }
        }
    }

}