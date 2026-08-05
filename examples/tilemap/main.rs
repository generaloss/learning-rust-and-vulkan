// 'main.rs'

pub mod tilemap;
pub mod entity;

use std::cmp::max;
use std::ops::{Add, AddAssign, Deref, Mul, Sub};
use glam::{Vec2, Vec3, Vec3Swizzles};
use noise::{Fbm, NoiseFn, Simplex};
use winit::error::EventLoopError;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use rand::random_range;
use winit::event::MouseButton;
use engine::application_context::{ContextBuilder, ContextManager, AppAdapter, ContextFields};
use engine::camera::CameraOrthographic;
use engine::sprite_batch::SpriteBatch;
use engine::texture::Texture;
use engine::input::{KeyCode};
use engine::texture_region::Region;
use crate::entity::{ComponentPosition, Entity};
use crate::tilemap::Tilemap;

const TILE_SIZE: f32 = 32.0;

const TILE_AIR: u32 = 0;
const TILE_GRASS: u32 = 1;
const TILE_DIRT: u32 = 2;
const TILE_STONE: u32 = 3;
const TILE_PLANKS: u32 = 4;

fn main() -> Result<(), EventLoopError> {
    let mut context1 = ContextBuilder::new()
        .title("Blazing Fast Engine")
        .size(1280, 720)
        .icon("assets/icon.png")
        .create();
    context1.set_app(BlazingFastApp::new("win 1".to_string()));

    let mut context2 = ContextBuilder::new()
        .title("Blazing Fast Engine")
        .size(1280, 720)
        .icon("assets/icon.png")
        .create();
    context2.set_app(BlazingFastApp::new("win 2".to_string()));

    let mut manager = ContextManager::new();
    manager.register(context1);
    manager.register(context2);
    manager.run()?;

    Ok(())
}

struct BlazingFastApp {
    name: String,

    batch: Option<SpriteBatch>,
    textures: Vec<Texture>,
    camera: CameraOrthographic,
    tile_map: Tilemap,

    player_pos: Vec2,
    player_speed: f32,
}

impl BlazingFastApp {
    fn new(name: String) -> Self {
        Self {
            name,
            batch: None,
            textures: Vec::new(),
            camera: CameraOrthographic::new(),
            tile_map: Tilemap::new(500, 100),

            player_pos: Vec2::new(250.0, 80.0).mul(TILE_SIZE),
            player_speed: 4.0,
        }
    }
}

impl AppAdapter for BlazingFastApp {
    fn init(&mut self, fields: &mut ContextFields) {
        self.camera.set_origin(Vec2::splat(0.5));
        self.camera.position.add_assign(Vec3::new(self.player_pos.x + 0.5 * TILE_SIZE, self.player_pos.y + 0.5 * TILE_SIZE, 0.0));

        let grass_texture  = Texture::from_path(&fields.vulkan, "assets/tiles/grass.png");
        let dirt_texture   = Texture::from_path(&fields.vulkan, "assets/tiles/dirt.png");
        let stone_texture  = Texture::from_path(&fields.vulkan, "assets/tiles/stone.png");
        let planks_texture = Texture::from_path(&fields.vulkan, "assets/tiles/planks.png");

        self.textures = vec![grass_texture, dirt_texture, stone_texture, planks_texture];

        let mut batch = SpriteBatch::new(&fields.vulkan, 100000);
        let texture_refs: Vec<&Texture> = self.textures.iter().collect();
        batch.set_textures(&texture_refs);

        self.batch = Some(batch);

        let mut fbm = Fbm::<Simplex>::new(42);

        fbm.frequency = 0.02;
        fbm.octaves = 4;

        for x in 0..self.tile_map.width {
            let noise_val = fbm.get([x as f64, 0.0]) + 0.5;
            let noise_height = (noise_val * 20.0).round() as usize + 60;

            self.tile_map.set_tile(x, noise_height, TILE_GRASS);

            let dirt_height = max(0, noise_height - 10);
            for y in dirt_height..noise_height {
                self.tile_map.set_tile(x, y, TILE_DIRT);
            }

            let stone_height = max(0, dirt_height);
            for y in 0..stone_height {
                self.tile_map.set_tile(x, y, TILE_STONE);
            }
        }

        fbm.octaves = 1;
        fbm.frequency = 0.4;

        let mut min_v = f32::MAX;
        let mut max_v = f32::MIN;

        self.tile_map.set_colors(|x, y| {
            let color_noise = fbm.get([x as f64 * 0.2, y as f64 * 0.2]) as f32;
            let b = color_noise + 0.5;
            min_v = min_v.min(b);
            max_v = max_v.max(b);
            [b, b, b, 1.0]
        });

        println!("min: {}, max: {}", min_v, max_v);

        // !!!!! DEBUG !!!!!
        let mut player = Entity::new(1)
            .with(Box::new(ComponentPosition { x: 500.25, y: -120.0 }));

        let binary_data: Vec<u8> = player.serialize_to_binary();

        println!("Размер бинарных данных: {} байт", binary_data.len());
        println!("Сырые байты: {:?}", binary_data);

        let loaded_player = Entity::deserialize_from_binary(&binary_data);
        println!("Сущность успешно загружена! Количество компонентов: {}", loaded_player.components.len());
    }

    fn update(&mut self, fields: &mut ContextFields) {
        let input = &fields.input;

        const SCALE_FACTOR: f32 = 1.01;
        const INV_SCALE_FACTOR: f32 = 1.0 / SCALE_FACTOR;

        let scroll = input.scroll_delta.y;

        if input.is_key_pressed(KeyCode::Minus) || scroll < 0.0 {
            self.camera.scale *= 1.0 + (SCALE_FACTOR - 1.0) * scroll.abs().max(1.0) * 5.0;
        }
        if input.is_key_pressed(KeyCode::Equal) || scroll > 0.0 {
            self.camera.scale *= 1.0 - (1.0 - INV_SCALE_FACTOR) * scroll.abs().max(1.0) * 5.0;
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

        if input.is_key_down(KeyCode::F11) {
            fields.toggle_fullscreen();
        }

        if input.is_key_up(KeyCode::Escape) {
            fields.should_close = true;
        }

        if input.is_button_pressed(MouseButton::Left) {
            let cx = input.position.x as f32;
            let cy = self.camera.height - input.position.y as f32;

            let scx = (cx - self.camera.width * 0.5) * self.camera.scale.x;
            let scy = (cy - self.camera.height * 0.5) * self.camera.scale.y;

            let x = scx + self.camera.position.x;
            let y = scy + self.camera.position.y;

            if x >= 0.0 && y >= 0.0 {
                let tile_x = (x / TILE_SIZE) as usize;
                let tile_y = (y / TILE_SIZE) as usize;

                if tile_x < self.tile_map.width && tile_y < self.tile_map.height {
                    self.tile_map.set_tile(tile_x, tile_y, TILE_STONE);
                }
            }
        }
        if input.is_button_pressed(MouseButton::Right) {
            let cx = input.position.x as f32;
            let cy = self.camera.height - input.position.y as f32;

            let scx = (cx - self.camera.width * 0.5) * self.camera.scale.x;
            let scy = (cy - self.camera.height * 0.5) * self.camera.scale.y;

            let x = scx + self.camera.position.x;
            let y = scy + self.camera.position.y;

            if x >= 0.0 && y >= 0.0 {
                let tile_x = (x / TILE_SIZE) as usize;
                let tile_y = (y / TILE_SIZE) as usize;

                if tile_x < self.tile_map.width && tile_y < self.tile_map.height {
                    self.tile_map.set_tile(tile_x, tile_y, 0);
                }
            }
        }
        // !!!!! DEBUG !!!!!
        if input.delta.x != 0.0 || input.delta.y != 0.0 {
            println!("Da-{} {} {}", self.name, input.delta.x, input.delta.y);
        }

        // camera gliding
        let camera_glide_dir = self.player_pos.add(0.5 * TILE_SIZE).sub(self.camera.position.xy()).mul(0.05);
        self.camera.position.add_assign(Vec3::new(camera_glide_dir.x, camera_glide_dir.y, 0.0));

        self.camera.update();
    }

    fn render(&mut self, _fields: &mut ContextFields, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if let Some(batch) = &mut self.batch {
            batch.begin();

            self.tile_map.iter(|x, y, tile_id, color| {
                if tile_id == 0 {
                    return;
                }

                let screen_x = x as f32 * TILE_SIZE;
                let screen_y = y as f32 * TILE_SIZE;

                let texture_id = tile_id - 1;

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