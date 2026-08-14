// 'vulkan_context.rs'

use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderingAttachmentInfo, RenderingInfo};
use vulkano::device::physical::{PhysicalDeviceType};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags, DeviceFeatures};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, PresentMode, SwapchainAcquireFuture};
use vulkano::sync::{self, GpuFuture};
use winit::window::Window;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::viewport::Viewport;

pub struct FrameInfo {
    pub image_index: u32,
    pub acquire_future: SwapchainAcquireFuture,
}

pub struct VulkanContext {
    pub instance: Arc<Instance>,
    pub surface: Arc<Surface>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<Image>>,
    pub image_views: Vec<Arc<ImageView>>,
    pub cb_allocator: Arc<StandardCommandBufferAllocator>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub image_format: Format,
}

impl VulkanContext {
    pub fn new(window: Arc<Window>) -> Self {
        // 1. Load library
        let library = VulkanLibrary::new()
            .expect("Failed to load Vulkan library");

        // 2. Instance extensions for the Window
        let required_extensions = Surface::required_extensions(&window).unwrap();

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
            .expect("Failed to create instance");

        // 3. Create Surface
        let surface = Surface::from_window(instance.clone(), window.clone())
            .expect("Failed to create surface");

        // 4. Выбираем физическое устройство
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // Ищем дискретную видеокарту с поддержкой графической очереди и swapchain
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| q.queue_flags.contains(QueueFlags::GRAPHICS) && p.surface_support(i as u32, &surface).unwrap_or(false))
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| {
                // Приоритет дискретным видеокартам
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("no suitable physical device found");

        // 5. Создаем логическое устройство
        let device_features = DeviceFeatures {
            dynamic_rendering: true,
            shader_sampled_image_array_non_uniform_indexing: true,
            ..DeviceFeatures::empty()
        };

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                enabled_features: device_features,
                ..Default::default()
            },
        ).expect("failed to create device");

        let queue = queues.next().unwrap();

        // 6. Создаем Swapchain
        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let image_format = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .expect("failed to get surface formats")[0].0;

        let window_size = window.inner_size();

        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count.max(2),
                image_format,
                image_extent: [window_size.width, window_size.height],
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha: surface_capabilities.supported_composite_alpha.into_iter().next().unwrap(),
                present_mode: PresentMode::Fifo,
                ..Default::default()
            },
        ).expect("failed to create swapchain");

        // 7. Создаем Image Views для картинок свопчейна
        let image_views = create_image_views(&images);

        // Аллокатор для командных буферов
        let cb_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));

        // 8. Инициализируем аллокатор памяти для буферов (выделяет память на GPU)
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        window.set_visible(true);

        Self {
            instance,
            surface,
            device,
            queue,
            swapchain,
            images,
            image_views,
            cb_allocator,
            memory_allocator,
            image_format,
        }
    }

    // Мгновенный для ОС ресайз
    pub fn resize_event(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: [width, height],
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to recreate swapchain during resize: {e}");
                return;
            }
        };

        self.swapchain = new_swapchain;
        self.images = new_images;
        self.image_views = create_image_views(&self.images);
    }

    pub fn begin_frame(&mut self, window_size: winit::dpi::PhysicalSize<u32>) -> Option<(AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, FrameInfo)> {
        // 1. Получаем индекс изображения
        let (image_index, suboptimal, acquire_future) =
            match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(vulkano::Validated::Error(vulkano::VulkanError::OutOfDate)) => {
                    self.resize_event(window_size.width, window_size.height);
                    return None; // Пропускаем кадр, пересоздав ресурсы
                }
                Err(e) => panic!("Failed to acquire next image: {e}"),
            };

        // Если свопчейн устарел, тоже безопасно пересоздаем его и пропускаем кадр
        if suboptimal {
            self.resize_event(window_size.width, window_size.height);
            return None;
        }

        // Беру реальный размер текстур текущего свопчейна, а не окна winit!
        let swapchain_extent = self.swapchain.image_extent();

        // depth
        let depth_image = ImageView::new_default(
            Image::new(
                self.memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::D32_SFLOAT,
                    extent: [window_size.width, window_size.height, 1],
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                    ..Default::default()
                },
            ).unwrap()
        ).unwrap();

        // 2. Билдим команды
        let mut builder = AutoCommandBufferBuilder::primary(
            self.cb_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    clear_value: Some([0.1, 0.2, 0.4, 1.0].into()),
                    load_op: vulkano::render_pass::AttachmentLoadOp::Clear,
                    store_op: vulkano::render_pass::AttachmentStoreOp::Store,
                    ..RenderingAttachmentInfo::image_view(self.image_views[image_index as usize].clone())
                })],
                depth_attachment: Some(RenderingAttachmentInfo {
                    load_op: vulkano::render_pass::AttachmentLoadOp::Clear,
                    store_op: vulkano::render_pass::AttachmentStoreOp::DontCare,
                    clear_value: Some(1.0.into()),
                    ..RenderingAttachmentInfo::image_view(depth_image)
                }),
                ..Default::default()
            })
            .unwrap();

        // Задаем область вывода под РЕАЛЬНЫЙ размер свопчейна
        builder
            .set_viewport(0, [Viewport {
                offset: [0.0, 0.0],
                extent: [swapchain_extent[0] as f32, swapchain_extent[1] as f32],
                depth_range: 0.0f32..=1.0f32,
            }].into_iter().collect())
            .unwrap()
            .set_scissor(0, [vulkano::pipeline::graphics::viewport::Scissor {
                offset: [0, 0],
                extent: swapchain_extent,
            }].into_iter().collect())
            .unwrap();

        let frame_info = FrameInfo {
            image_index,
            acquire_future,
        };

        Some((builder, frame_info))
    }

    pub fn end_frame(&mut self, mut builder: AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
                     frame_info: FrameInfo, window_size: winit::dpi::PhysicalSize<u32>) {
        builder.end_rendering().unwrap();
        let command_buffer = builder.build().unwrap();

        let future = sync::now(self.device.clone())
            .join(frame_info.acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), frame_info.image_index),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                future.wait(None).unwrap();
            }
            // Ловим И OutOfDate, И SurfaceLost
            Err(vulkano::Validated::Error(vulkano::VulkanError::OutOfDate | vulkano::VulkanError::SurfaceLost)) => {
                self.resize_event(window_size.width, window_size.height);
            }
            Err(e) => {
                println!("Failed to flush future: {e}");
            }
        }
    }

}

fn create_image_views(images: &[Arc<Image>]) -> Vec<Arc<ImageView>> {
    images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).unwrap())
        .collect()
}