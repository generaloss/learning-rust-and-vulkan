// 'texture.rs'

use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::format::Format;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::sync::GpuFuture;
use crate::vulkan_context::VulkanContext;

pub struct Texture {
    pub image: Arc<Image>,
    pub image_view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
}

impl Texture {
    pub fn from_path(vulkan_context: &VulkanContext, path: &str) -> Self {
        // Load image data
        let rgba_image = image::open(path)
            .expect("Failed to open image file!")
            .to_rgba8();

        let (width, height) = rgba_image.dimensions();
        let image_data = rgba_image.into_raw();

        let image = Image::new(
            vulkan_context.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            }
        ).unwrap();

        // Загружаем пиксели в текстуру (простейший способ через upload_from_iterator)
        // Для этого используем вспомогательный буфер, который vulkano сделает под капотом
        let mut upload_builder = AutoCommandBufferBuilder::primary(
            vulkan_context.cb_allocator.clone(),
            vulkan_context.queue.queue_family_index(),
            vulkano::command_buffer::CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        // Create staging Buffer to copy the data to GPU
        let staging_buffer = Buffer::from_iter(
            vulkan_context.memory_allocator.clone(),
            BufferCreateInfo { usage: BufferUsage::TRANSFER_SRC, ..Default::default() },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            image_data,
        ).unwrap();

        upload_builder.copy_buffer_to_image(vulkano::command_buffer::CopyBufferToImageInfo::buffer_image(
            staging_buffer,
            image.clone(),
        )).unwrap();

        let upload_cb = upload_builder.build().unwrap();
        vulkano::sync::now(vulkan_context.device.clone())
            .then_execute(vulkan_context.queue.clone(), upload_cb).unwrap()
            .then_signal_fence_and_flush().unwrap()
            .wait(None).unwrap(); // Ждем завершения загрузки текстуры

        let image_view = ImageView::new_default(image.clone()).unwrap();

        // Create Sampler
        let sampler = Sampler::new(
            vulkan_context.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            }
        ).unwrap();

        Self {
            image,
            image_view,
            sampler,
        }
    }
}