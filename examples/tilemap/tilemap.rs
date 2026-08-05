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

    #[inline]
    pub fn tile_at(&self, x: usize, y: usize) -> u32 {
        let index = y * self.width + x;
        self.tiles[index]
    }

    #[inline]
    pub fn set_tile(&mut self, x: usize, y: usize, tile_id: u32) {
        let index = y * self.width + x;
        self.tiles[index] = tile_id;
    }

    #[inline]
    pub fn color_at(&self, x: usize, y: usize) -> [f32; 4] {
        let index = y * self.width + x;
        self.colors[index]
    }

    #[inline]
    pub fn set_color(&mut self, x: usize, y: usize, color: [f32; 4]) {
        let index = y * self.width + x;
        self.colors[index] = color;
    }

    pub fn set_tiles<F>(&mut self, mut tile_selector: F) where F: FnMut(usize, usize) -> u32 {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_tile(x, y, tile_selector(x, y));
            }
        }
    }

    pub fn set_colors<F>(&mut self, mut color_selector: F) where F: FnMut(usize, usize) -> [f32; 4] {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_color(x, y, color_selector(x, y));
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