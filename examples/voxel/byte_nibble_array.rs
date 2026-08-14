use crate::chunk::{AREA, SIZE, VOLUME};

pub struct ByteNibbleArray3D {
    pub array: [u8; VOLUME]
}

impl ByteNibbleArray3D {

    pub fn init(default: u8) -> Self {
        Self {
            array: [default; VOLUME]
        }
    }

    pub fn from_array(array: [u8; VOLUME]) -> Self {
        Self {
            array
        }
    }

    #[inline]
    pub fn is_in_bounds(x: usize, y: usize, z: usize) -> bool {
        x < SIZE && y < SIZE && z < SIZE
    }

    #[inline]
    pub fn index(x: usize, y: usize, z: usize) -> Option<usize> {
        if Self::is_in_bounds(x, y, z) {
            Some(x + y * SIZE + z * AREA)
        } else {
            None
        }
    }

    #[inline]
    pub fn index_unchecked(x: usize, y: usize, z: usize) -> usize {
        x + y * SIZE + z * AREA
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<u8> {
        Self::index(x, y, z).map(|index| self.array[index])
    }

    #[inline]
    pub fn get_or_default(&self, x: usize, y: usize, z: usize, default: u8) -> u8 {
        self.get(x, y, z).unwrap_or(default)
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: u8) -> bool {
        if let Some(index) = Self::index(x, y, z) {
            self.array[index] = value;
            true
        } else {
            false
        }
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, x: usize, y: usize, z: usize) -> u8 {
        let index = Self::index_unchecked(x, y, z);
        self.array[index]
    }

    #[inline]
    pub unsafe fn set_unchecked(&mut self, x: usize, y: usize, z: usize, value: u8) {
        let index = Self::index_unchecked(x, y, z);
        self.array[index] = value;
    }

}