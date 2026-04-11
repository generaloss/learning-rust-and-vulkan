use ash;
use ash::vk;
use ash::khr::surface;
use ash::khr::swapchain;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub surface_loader: surface::Instance,
    pub surface: vk::SurfaceKHR,

    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub queue: vk::Queue,
    pub queue_family_index: u32,

    pub swapchain_loader: swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,

    pub extent: vk::Extent2D,

    pub image_available: vk::Semaphore,
    pub render_finished: vk::Semaphore,
    pub in_flight: vk::Fence,

    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
}

impl VulkanContext {

    pub fn new(window: &Window) -> Self {
        unsafe {
            // entry
            let entry = ash::Entry::linked();

            // instance
            let app_info = vk::ApplicationInfo::default()
                .api_version(vk::API_VERSION_1_3);
            let raw_display_handle = window.display_handle().unwrap().as_raw();
            let extensions = ash_window::enumerate_required_extensions(raw_display_handle).unwrap();

            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default()
                    .application_info(&app_info)
                    .enabled_extension_names(extensions),
                None
            ).unwrap();

            // surface
            let raw_window_handle = window.window_handle().unwrap().as_raw();

            let surface = ash_window::create_surface(
                &entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None
            ).unwrap();

            // surface loader
            let surface_loader = surface::Instance::new(&entry, &instance);

            // physical device
            let physical_device = instance
                .enumerate_physical_devices()
                .unwrap()[0];

            // queue family index
            let queue_family_index = instance.get_physical_device_queue_family_properties(physical_device)
                .iter()
                .enumerate()
                .find(|(index, qfp)| {
                    let supports_graphics = qfp.queue_flags.contains(vk::QueueFlags::GRAPHICS);

                    let supports_surface = surface_loader
                        .get_physical_device_surface_support(
                            physical_device,
                            *index as u32,
                            surface
                        ).unwrap();

                    (supports_graphics && supports_surface)
                })
                .map(|(index, _)| index as u32)
                .expect("No suitable queue family found");

            // device
            let device_extensions = [ swapchain::NAME.as_ptr() ];
            let priorities = [ 1.0 ];

            let device = instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::default()
                    .enabled_extension_names(&device_extensions)
                    .queue_create_infos(&[
                        vk::DeviceQueueCreateInfo::default()
                            .queue_family_index(queue_family_index)
                            .queue_priorities(&priorities)
                    ]),
                None
            ).unwrap();

            // queue
            let queue = device.get_device_queue(queue_family_index, 0);

            // command pool
            let command_pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue_family_index),
                None
            ).unwrap();

            // command buffer
            let command_buffer = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1)
            ).unwrap()[0];

            // swapchain loader
            let swapchain_loader = swapchain::Device::new(&instance, &device);

            // swapchain
            let surface_format = surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()[0];

            // let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: 1280, // width: window_inner_size.width,
                height: 720, // height: window_inner_size.height,
            };

            let capabilities = surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap();

            let swapchain = swapchain_loader.create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(surface)
                    .min_image_count(2)
                    .image_format(surface_format.format)
                    .image_color_space(surface_format.color_space)
                    .image_extent(extent)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_array_layers(1)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(capabilities.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO) // vsync
                    .clipped(true),
                None
            ).unwrap();

            // images
            let images = swapchain_loader.get_swapchain_images(swapchain).unwrap();

            // image views
            let image_views = images.iter()
                .map(|&image| {
                    device.create_image_view(
                        &vk::ImageViewCreateInfo::default()
                            .image(image)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(surface_format.format)
                            .subresource_range(
                                vk::ImageSubresourceRange::default()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .level_count(1)
                                    .layer_count(1)
                            ),
                        None
                    ).unwrap()
                })
                .collect::<Vec<_>>();

            // render pass
            let attachment_desc = vk::AttachmentDescription::default()
                .format(surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

            let subpass_desc = vk::SubpassDescription::default()
                .color_attachments(&[
                    vk::AttachmentReference {
                        attachment: 0,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
                    }
                ]);

            let dependency = vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

            let render_pass = device.create_render_pass(
                &vk::RenderPassCreateInfo::default()
                    .attachments(&[attachment_desc])
                    .subpasses(&[subpass_desc])
                    .dependencies(&[dependency]),
                None
            ).unwrap();

            // framebuffers
            let framebuffers = image_views.iter()
                .map(|&image_view| {
                    device.create_framebuffer(
                        &vk::FramebufferCreateInfo::default()
                            .render_pass(render_pass)
                            .attachments(&[image_view])
                            .width(extent.width)
                            .height(extent.height)
                            .layers(1),
                        None
                    ).unwrap()
                })
                .collect::<Vec<_>>();

            // image available
            let semaphore_info = vk::SemaphoreCreateInfo::default();
            let image_available = device.create_semaphore(&semaphore_info, None).unwrap();

            // render finished
            let render_finished = device.create_semaphore(&semaphore_info, None).unwrap();

            // in flight
            let fence_info = vk::FenceCreateInfo::default()
                .flags(vk::FenceCreateFlags::SIGNALED);

            let in_flight = device.create_fence(&fence_info, None).unwrap();

            Self {
                entry,
                instance,
                surface_loader,
                surface,

                device,
                physical_device,
                queue,
                queue_family_index,

                swapchain_loader,
                swapchain,
                images,
                image_views,

                render_pass,
                framebuffers,

                extent,

                image_available,
                render_finished,
                in_flight,

                command_pool,
                command_buffer,
            }
        }
    }

    pub fn render(&mut self) {
        unsafe {
            self.device.wait_for_fences(&[self.in_flight], true, u64::MAX).unwrap();
            self.device.reset_fences(&[self.in_flight]).unwrap();

            let (image_index, _) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available,
                vk::Fence::null()
            ).unwrap();

            let command_buffer = self.command_buffer;
            let command_buffers = &[command_buffer];

            self.device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty()).unwrap();

            self.device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::default()
            ).unwrap();

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.1, 0.2, 0.3, 1.0]
                }
            }];

            self.device.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(self.render_pass)
                    .framebuffer(self.framebuffers[image_index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent,
                    })
                    .clear_values(&clear_values),
                vk::SubpassContents::INLINE
            );

            self.device.cmd_end_render_pass(command_buffer);

            self.device.end_command_buffer(command_buffer).unwrap();

            let wait_semaphores = &[self.image_available];
            let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = &[self.render_finished];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(wait_stages)
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            self.device.queue_submit(self.queue, &[submit_info], self.in_flight).unwrap();

            let swapchains = &[self.swapchain];
            let image_indices = &[image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            self.swapchain_loader.queue_present(self.queue, &present_info).unwrap();
        }
    }

}