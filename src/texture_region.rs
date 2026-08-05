// texture_region

pub struct Region {
    pub u1: f32,
    pub v1: f32,
    pub u2: f32,
    pub v2: f32,
}

impl Region {
    
    #[inline]
    pub const fn new(u1: f32, v1: f32, u2: f32, v2: f32) -> Self {
        Self { u1, v1, u2, v2, }
    }
    
    pub const DEFAULT:Self = Self::new(0.0, 0.0, 1.0, 1.0);
    
}
