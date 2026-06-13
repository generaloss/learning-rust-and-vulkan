// 'main.rs'

pub mod engine;
pub mod tilemap;

use std::ops::{AddAssign, Mul, Sub};
use glam::{Vec2, Vec3, Vec3Swizzles};
use noise::{Fbm, NoiseFn, Perlin};
use winit::error::EventLoopError;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use rand::random_range;

use crate::engine::application_context::{ContextBuilder, ContextManager, AppAdapter, ContextFields};
use crate::engine::camera::CameraOrthographic;
use crate::engine::sprite_batch::SpriteBatch;
use crate::engine::texture::Texture;
use crate::engine::input::{KeyCode};
use crate::engine::texture_region::Region;
use crate::tilemap::Tilemap;

const TILE_SIZE: f32 = 16.0;

const TILE_GRASS: u32 = 0;
const TILE_DIRT: u32 = 1;
const TILE_STONE: u32 = 2;
const TILE_PLANKS: u32 = 3;

fn main() -> Result<(), EventLoopError> {
    let mut context = ContextBuilder::new()
        .title("Blazing Fast Engine")
        .size(1280, 720)
        .icon("assets/icon.png")
        .create();
    context.set_app(BlazingFastApp::new());

    let mut manager = ContextManager::new();
    manager.register(context);
    manager.run()?;

    Ok(())
}

struct BlazingFastApp {
    batch: Option<SpriteBatch>,
    textures: Vec<Texture>,
    camera: CameraOrthographic,
    tile_map: Tilemap,

    player_pos: Vec2,
    player_speed: f32,
}

impl BlazingFastApp {
    fn new() -> Self {
        Self {
            batch: None,
            textures: Vec::new(),
            camera: CameraOrthographic::new(),
            tile_map: Tilemap::new(300, 300),

            player_pos: Vec2::new(25.0, 20.0).mul(TILE_SIZE),
            player_speed: 4.0,
        }
    }
}

impl AppAdapter for BlazingFastApp {
    fn init(&mut self, fields: &mut ContextFields) {
        self.camera.set_origin(Vec2::splat(0.5));

        let grass_texture  = Texture::from_path(&fields.vulkan, "assets/tiles/grass.png");
        let dirt_texture   = Texture::from_path(&fields.vulkan, "assets/tiles/dirt.png");
        let stone_texture  = Texture::from_path(&fields.vulkan, "assets/tiles/stone.png");
        let planks_texture = Texture::from_path(&fields.vulkan, "assets/tiles/planks.png");

        self.textures = vec![grass_texture, dirt_texture, stone_texture, planks_texture];

        let mut batch = SpriteBatch::new(&fields.vulkan, 1000000);
        let texture_refs: Vec<&Texture> = self.textures.iter().collect();
        batch.set_textures(&texture_refs);

        self.batch = Some(batch);

        let width = self.tile_map.width;
        let height = self.tile_map.height;


        let mut fbm = Fbm::<Perlin>::new(42);

        fbm.frequency = 0.05;
        fbm.octaves = 4;

        self.tile_map.set_tiles(|x: usize, y: usize| {
            if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                return TILE_STONE;
            }

            if x >= 22 && x <= 28 && y >= 22 && y <= 28 {
                if x == 22 || x == 28 || y == 22 || y == 28 {
                    return if x == 25 && y == 22 { TILE_DIRT } else { TILE_STONE };
                }
                return TILE_PLANKS;
            }

            let noise_val = fbm.get([x as f64, y as f64]);

            if noise_val < -0.2 {
                TILE_STONE
            } else if noise_val < 0.3 {
                TILE_GRASS
            } else {
                TILE_DIRT
            }
        });

        fbm.octaves = 1;
        fbm.frequency = 0.4;

        self.tile_map.set_colors(|x, y| {
            let color_noise = fbm.get([x as f64 * 0.2, y as f64 * 0.2]) as f32;
            let b = 0.5 + (color_noise * 0.5);
            [b, b, b, 1.0]
        });
    }

    fn update(&mut self, fields: &mut ContextFields) {
        let input = &fields.input;

        const SCALE_FACTOR: f32 = 1.01;
        const INV_SCALE_FACTOR: f32 = 1.0 / SCALE_FACTOR;

        if input.is_key_pressed(KeyCode::Minus) {
            self.camera.scale *= SCALE_FACTOR;
        }
        if input.is_key_pressed(KeyCode::Equal) {
            self.camera.scale *= INV_SCALE_FACTOR;
        }

        if input.is_key_pressed(KeyCode::KeyA)  {
            self.player_pos.x -= self.player_speed;
        }
        if input.is_key_pressed(KeyCode::KeyD) {
            self.player_pos.x += self.player_speed;
        }
        if input.is_key_pressed(KeyCode::KeyW) {
            self.player_pos.y += self.player_speed;
        }
        if input.is_key_pressed(KeyCode::KeyS)  {
            self.player_pos.y -= self.player_speed;
        }

        if input.is_key_down(KeyCode::Space) {
            self.player_pos.x = random_range(1..self.tile_map.width - 1) as f32 * TILE_SIZE;
            self.player_pos.y = random_range(1..self.tile_map.height - 1) as f32 * TILE_SIZE;
            println!("Teleport! New position: {}, {}", self.player_pos.x, self.player_pos.y);
        }

        let camera_glide_dir = self.player_pos.sub(self.camera.position.xy()).mul(0.05);
        self.camera.position.add_assign(Vec3::new(camera_glide_dir.x, camera_glide_dir.y, 0.0));

        if input.is_key_up(KeyCode::Escape) {
            fields.should_close = true;
        }

        self.camera.update();
    }

    fn render(&mut self, _fields: &mut ContextFields, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if let Some(batch) = &mut self.batch {
            batch.begin();

            self.tile_map.iter(|x, y, texture_id, color| {
                let screen_x = x as f32 * TILE_SIZE;
                let screen_y = y as f32 * TILE_SIZE;

                batch.draw_quad_c(screen_x, screen_y, TILE_SIZE, TILE_SIZE, texture_id, color);
            });

            batch.draw_quad_r(self.player_pos.x, self.player_pos.y, TILE_SIZE, TILE_SIZE, TILE_STONE, Region::new(-0.5, -0.5, 1.5, 1.5));

            batch.end(builder, self.camera.combined.to_cols_array_2d());
        }
    }

    fn resize(&mut self, _fields: &mut ContextFields, width: u32, height: u32) {
        self.camera.resize(width as f32, height as f32);
    }

    fn shutdown(&mut self) {
        println!("Shutdown");
    }
}