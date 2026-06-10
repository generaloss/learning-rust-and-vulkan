// 'main.rs'

pub mod engine;

use winit::error::EventLoopError;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use rand::random_range;

use crate::engine::application_context::{ContextBuilder, ContextManager, AppAdapter};
use crate::engine::vulkan_context::VulkanContext;
use crate::engine::camera::CameraOrthographic;
use crate::engine::sprite_batch::SpriteBatch;
use crate::engine::texture::Texture;

const MAP_WIDTH: usize = 50;
const MAP_HEIGHT: usize = 50;
const TILE_SIZE: f32 = 32.0;

const TILE_GRASS: u32 = 0;
const TILE_DIRT: u32 = 1;
const TILE_STONE: u32 = 2;
const TILE_PLANKS: u32 = 3;

fn main() -> Result<(), EventLoopError> {
    let mut context = ContextBuilder::new()
        .title("Blazing Fast Engine")
        .size(720, 720)
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
    tile_map: Vec<u32>,
}

impl BlazingFastApp {
    fn new() -> Self {
        Self {
            batch: None,
            textures: Vec::new(),
            camera: CameraOrthographic::new(),
            tile_map: vec![TILE_GRASS; MAP_WIDTH * MAP_HEIGHT],
        }
    }
}

impl AppAdapter for BlazingFastApp {
    fn init(&mut self, vulkan_context: &mut VulkanContext) {
        let grass_texture  = Texture::from_path(vulkan_context, "assets/tiles/grass.png");
        let dirt_texture   = Texture::from_path(vulkan_context, "assets/tiles/dirt.png");
        let stone_texture  = Texture::from_path(vulkan_context, "assets/tiles/stone.png");
        let planks_texture = Texture::from_path(vulkan_context, "assets/tiles/planks.png");

        self.textures = vec![grass_texture, dirt_texture, stone_texture, planks_texture];

        let mut batch = SpriteBatch::new(vulkan_context, 10000);
        let texture_refs: Vec<&Texture> = self.textures.iter().collect();
        batch.set_textures(&texture_refs);

        self.batch = Some(batch);

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let index = y * MAP_WIDTH + x;

                if x == 0 || y == 0 || x == MAP_WIDTH - 1 || y == MAP_HEIGHT - 1 {
                    self.tile_map[index] = TILE_STONE;
                }
                else if x >= 22 && x <= 28 && y >= 22 && y <= 28 {
                    if x == 22 || x == 28 || y == 22 || y == 28 {
                        if x == 25 && y == 22 {
                            self.tile_map[index] = TILE_DIRT;
                        } else {
                            self.tile_map[index] = TILE_STONE;
                        }
                    } else {
                        self.tile_map[index] = TILE_PLANKS;
                    }
                }
                else if x == 25 || y == 25 {
                    self.tile_map[index] = TILE_DIRT;
                }
                else {
                    if random_range(0..5) == 0 {
                        self.tile_map[index] = TILE_DIRT;
                    } else {
                        self.tile_map[index] = TILE_GRASS;
                    }
                }
            }
        }
    }

    fn render(&mut self, _vulkan_context: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if let Some(batch) = &mut self.batch {
            batch.begin();

            for y in 0..MAP_HEIGHT {
                for x in 0..MAP_WIDTH {
                    let index = y * MAP_WIDTH + x;
                    let texture_id = self.tile_map[index];

                    let screen_x = x as f32 * TILE_SIZE;
                    let screen_y = y as f32 * TILE_SIZE;

                    batch.draw_quad(screen_x, screen_y, TILE_SIZE, TILE_SIZE, texture_id);
                }
            }

            batch.end(builder, self.camera.combined.to_cols_array_2d());
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width as f32, height as f32);
    }

    fn shutdown(&mut self) {
        println!("Shutdown");
    }
}