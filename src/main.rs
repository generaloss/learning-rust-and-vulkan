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
}

impl BlazingFastApp {
    fn new() -> Self {
        Self {
            batch: None,
            textures: Vec::new(),
            camera: CameraOrthographic::new(),
        }
    }
}

impl AppAdapter for BlazingFastApp {
    fn init(&mut self, vulkan_context: &mut VulkanContext) {
        // 1. Просто последовательно загружаем наши изображения с диска
        let texture_1 = Texture::from_path(vulkan_context, "assets/image_1.jpg");
        let texture_2 = Texture::from_path(vulkan_context, "assets/image_2.jpg");
        let texture_3 = Texture::from_path(vulkan_context, "assets/image_3.jpg");
        let texture_4 = Texture::from_path(vulkan_context, "assets/image_4.jpg");
        let texture_5 = Texture::from_path(vulkan_context, "assets/image_5.jpg");

        // Сохраняем владение текстурами в структуре приложения
        self.textures = vec![texture_1, texture_2, texture_3, texture_4, texture_5];

        // 2. Создаем наш изолированный SpriteBatch
        let mut batch = SpriteBatch::new(vulkan_context, 10000);

        // 3. Собираем массив ссылок на текстуры и скармливаем его бетчу
        let texture_refs: Vec<&Texture> = self.textures.iter().collect();
        batch.set_textures(&texture_refs);

        self.batch = Some(batch);
    }

    fn render(&mut self, _vulkan: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if let Some(batch) = &mut self.batch {
            batch.begin();

            // Массированная отрисовка сетки случайных квадов
            for i in 0..50 {
                for j in 0..50 {
                    batch.draw_quad(i as f32 * 20.0, j as f32 * 20.0, 20.0, 20.0, random_range(0..5));
                }
            }

            // Отрисовка крупных перекрывающих спрайтов поверх сетки
            batch.draw_quad(250.0, 50.0, 150.0, 150.0, 1);  // image_2
            batch.draw_quad(100.0, 300.0, 400.0, 200.0, 2); // image_3
            batch.draw_quad(50.0, 500.0, 100.0, 100.0, 4);  // image_5

            // Завершаем рендеринг кадра, передавая только матрицу проекции камеры
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