pub struct Tilemap {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<u32>,
    pub colors: Vec<[f32; 4]>,
}

impl Tilemap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![0; width * height],
            colors: vec![[1.0, 1.0, 1.0, 1.0]; width * height],
        }
    }

    pub fn set_tiles<F>(&mut self, mut tile_selector: F) where F: FnMut(usize, usize) -> u32 {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = y * self.width + x;
                self.tiles[index] = tile_selector(x, y);
            }
        }
    }

    pub fn set_colors<F>(&mut self, mut color_selector: F) where F: FnMut(usize, usize) -> [f32; 4] {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = y * self.width + x;
                self.colors[index] = color_selector(x, y);
            }
        }
    }

    pub fn iter<F>(&self, mut consumer: F) where F: FnMut(usize, usize, u32, [f32; 4]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = y * self.width + x;
                let tile_id = self.tiles[index];
                let color = self.colors[index];
                consumer(x, y, tile_id, color);
            }
        }
    }

}