// 'vulkan_context.rs'

use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo};
use vulkano::device::physical::{PhysicalDeviceType};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags, DeviceFeatures};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, PresentMode};
use vulkano::sync::{self, GpuFuture};
use winit::window::Window;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::viewport::Viewport;
use crate::engine::application_context::AppAdapter;

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
    pub image_format: vulkano::format::Format,
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
        let mut device_features = DeviceFeatures {
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

    pub fn render(&mut self, window: &Window, app: &mut Box<dyn AppAdapter>) {
        // 1. Получаем индекс изображения
        let (image_index, suboptimal, acquire_future) =
            match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(vulkano::Validated::Error(vulkano::VulkanError::OutOfDate)) => {
                    // Если вдруг пропустили ресайз, пересоздаем по текущему размеру окна
                    let size = window.inner_size();
                    self.resize_event(size.width, size.height);
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {e}"),
            };

        if suboptimal {
            let size = window.inner_size();
            self.resize_event(size.width, size.height);
        }

        // 2. Билдим команды
        let mut builder = AutoCommandBufferBuilder::primary(
            self.cb_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        let window_size = window.inner_size();

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    clear_value: Some([0.1, 0.2, 0.4, 1.0].into()),
                    load_op: vulkano::render_pass::AttachmentLoadOp::Clear,
                    store_op: vulkano::render_pass::AttachmentStoreOp::Store,
                    ..RenderingAttachmentInfo::image_view(self.image_views[image_index as usize].clone())
                })],
                ..Default::default()
            })
            .unwrap();

        // --- НАЧАЛО НОВЫХ КОМАНД ОТРИСОВКИ ---

        // Задаем область вывода под текущий размер окна
        builder
            .set_viewport(0, [Viewport {
                offset: [0.0, 0.0],
                extent: [window_size.width as f32, window_size.height as f32],
                depth_range: 0.0f32..=1.0f32,
            }].into_iter().collect())
            .unwrap()
            .set_scissor(0, [vulkano::pipeline::graphics::viewport::Scissor {
                offset: [0, 0],
                extent: [window_size.width, window_size.height],
            }].into_iter().collect())
            .unwrap();

        // ====================================================
        // ДЕЛЕГИРОВАНИЕ: Передаем управление игре!
        // Она запишет сюда свои бинды конвейеров и вызовы draw()
        app.render(self, &mut builder);
        // ====================================================

        // --- КОНЕЦ КОМАНД ОТРИСОВКИ ---

        builder.end_rendering().unwrap();
        let command_buffer = builder.build().unwrap();

        // Уведомляем систему, что мы вот-вот выведем кадр на экран (важно для плавности в winit)
        window.pre_present_notify();

        // 3. Отправляем в очередь девайса
        let future = sync::now(self.device.clone())
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                future.wait(None).unwrap(); // Ждем завершения кадра
            }
            Err(vulkano::Validated::Error(vulkano::VulkanError::OutOfDate)) => {
                let size = window.inner_size();
                self.resize_event(size.width, size.height);
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