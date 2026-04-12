use ash;
use ash::vk;
use ash::khr::{surface, swapchain};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct FrameData {
    cmd: vk::CommandBuffer,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    fence: vk::Fence,
}

struct OldSwapchain {
    swapchain: vk::SwapchainKHR,
    image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
}

pub struct VulkanContext {
    needs_resize: bool,

    entry: ash::Entry,
    instance: ash::Instance,
    surface_loader: surface::Instance,
    surface: vk::SurfaceKHR,

    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queue: vk::Queue,
    queue_family_index: u32,

    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,

    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    extent: vk::Extent2D,

    frames: Vec<FrameData>,
    current_frame: usize,

    command_pool: vk::CommandPool,

    old_swapchains: Vec<OldSwapchain>,
}

impl VulkanContext {

    pub fn new(window: &Window) -> Self {
        unsafe {
            let entry = ash::Entry::linked();

            let app_info = vk::ApplicationInfo::default()
                .api_version(vk::API_VERSION_1_3);

            let raw_display = window.display_handle().unwrap().as_raw();
            let extensions = ash_window::enumerate_required_extensions(raw_display).unwrap();

            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default()
                    .application_info(&app_info)
                    .enabled_extension_names(extensions),
                None
            ).unwrap();

            let raw_window = window.window_handle().unwrap().as_raw();

            let surface = ash_window::create_surface(
                &entry,
                &instance,
                raw_display,
                raw_window,
                None
            ).unwrap();

            let surface_loader = surface::Instance::new(&entry, &instance);

            let physical_device = instance.enumerate_physical_devices().unwrap()[0];

            let queue_family_index = instance
                .get_physical_device_queue_family_properties(physical_device)
                .iter()
                .enumerate()
                .find(|(i, q)| {
                    q.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && surface_loader
                        .get_physical_device_surface_support(physical_device, *i as u32, surface)
                        .unwrap()
                })
                .map(|(i, _)| i as u32)
                .unwrap();

            let priorities = [1.0];
            let device = instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::default()
                    .enabled_extension_names(&[swapchain::NAME.as_ptr()])
                    .queue_create_infos(&[
                        vk::DeviceQueueCreateInfo::default()
                            .queue_family_index(queue_family_index)
                            .queue_priorities(&priorities)
                    ]),
                None
            ).unwrap();

            let queue = device.get_device_queue(queue_family_index, 0);

            let command_pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue_family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None
            ).unwrap();

            let cmd_buffers = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32)
            ).unwrap();

            let mut frames = Vec::new();

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                let semaphore_info = vk::SemaphoreCreateInfo::default();
                let fence_info = vk::FenceCreateInfo::default()
                    .flags(vk::FenceCreateFlags::SIGNALED);

                frames.push(FrameData {
                    cmd: cmd_buffers[i],
                    image_available: device.create_semaphore(&semaphore_info, None).unwrap(),
                    render_finished: device.create_semaphore(&semaphore_info, None).unwrap(),
                    fence: device.create_fence(&fence_info, None).unwrap(),
                });
            }

            let swapchain_loader = swapchain::Device::new(&instance, &device);

            let mut ctx = Self {
                needs_resize: false,

                entry,
                instance,
                surface_loader,
                surface,

                device,
                physical_device,
                queue,
                queue_family_index,

                swapchain_loader,
                swapchain: vk::SwapchainKHR::null(),

                images: vec![],
                image_views: vec![],
                framebuffers: vec![],
                render_pass: vk::RenderPass::null(),
                extent: vk::Extent2D::default(),

                frames,
                current_frame: 0,

                command_pool,

                old_swapchains: vec![],
            };

            ctx.create_swapchain(window, vk::SwapchainKHR::null());

            ctx
        }
    }

    fn create_swapchain(&mut self, window: &Window, old: vk::SwapchainKHR) {
        unsafe {
            let caps = self.surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
                .unwrap();

            let format = self.surface_loader
                .get_physical_device_surface_formats(self.physical_device, self.surface)
                .unwrap()[0];

            let size = window.inner_size();

            let extent = if caps.current_extent.width != u32::MAX {
                caps.current_extent
            } else {
                vk::Extent2D {
                    width: size.width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
                    height: size.height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
                }
            };

            self.extent = extent;

            let image_count = (caps.min_image_count + 1)
                .min(caps.max_image_count.max(caps.min_image_count + 1));

            let new_swapchain = self.swapchain_loader.create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(self.surface)
                    .min_image_count(image_count)
                    .image_format(format.format)
                    .image_color_space(format.color_space)
                    .image_extent(extent)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_array_layers(1)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(caps.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .clipped(true)
                    .old_swapchain(old),
                None
            ).unwrap();

            if old != vk::SwapchainKHR::null() {
                self.old_swapchains.push(OldSwapchain {
                    swapchain: old,
                    image_views: std::mem::take(&mut self.image_views),
                    framebuffers: std::mem::take(&mut self.framebuffers),
                    render_pass: self.render_pass,
                });
            }

            self.swapchain = new_swapchain;

            self.images = self.swapchain_loader.get_swapchain_images(self.swapchain).unwrap();

            self.image_views = self.images.iter().map(|&img| {
                self.device.create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .image(img)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(format.format)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .level_count(1)
                                .layer_count(1)
                        ),
                    None
                ).unwrap()
            }).collect();

            let attachment = vk::AttachmentDescription::default()
                .format(format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

            let color_ref = [vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            }];

            let subpass = vk::SubpassDescription::default()
                .color_attachments(&color_ref);

            self.render_pass = self.device.create_render_pass(
                &vk::RenderPassCreateInfo::default()
                    .attachments(&[attachment])
                    .subpasses(&[subpass]),
                None
            ).unwrap();

            self.framebuffers = self.image_views.iter().map(|&view| {
                self.device.create_framebuffer(
                    &vk::FramebufferCreateInfo::default()
                        .render_pass(self.render_pass)
                        .attachments(&[view])
                        .width(extent.width)
                        .height(extent.height)
                        .layers(1),
                    None
                ).unwrap()
            }).collect();
        }
    }

    fn cleanup_old(&mut self) {
        unsafe {
            for old in self.old_swapchains.drain(..) {
                for fb in old.framebuffers {
                    self.device.destroy_framebuffer(fb, None);
                }
                for view in old.image_views {
                    self.device.destroy_image_view(view, None);
                }
                self.device.destroy_render_pass(old.render_pass, None);
                self.swapchain_loader.destroy_swapchain(old.swapchain, None);
            }
        }
    }

    pub fn resize_event(&mut self) {
        self.needs_resize = true;
    }

    pub fn render(&mut self, window: &Window) {
        unsafe {
            let frame = &self.frames[self.current_frame];

            let (image_index, _) = match self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                frame.image_available,
                vk::Fence::null()
            ) {
                Ok(r) => r,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.create_swapchain(window, self.swapchain);
                    return;
                }
                Err(e) => panic!("{:?}", e),
            };

            self.device.wait_for_fences(&[frame.fence], true, u64::MAX).unwrap();
            self.device.reset_fences(&[frame.fence]).unwrap();

            let cmd = frame.cmd;

            self.device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty()).unwrap();

            self.device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default()).unwrap();

            let clear = [vk::ClearValue {
                color: vk::ClearColorValue { float32: [0.1, 0.2, 0.3, 1.0] }
            }];

            self.device.cmd_begin_render_pass(
                cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(self.render_pass)
                    .framebuffer(self.framebuffers[image_index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent,
                    })
                    .clear_values(&clear),
                vk::SubpassContents::INLINE
            );

            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd).unwrap();

            let wait_semaphores = [frame.image_available];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [frame.render_finished];
            let cmd_bufs = [cmd];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&cmd_bufs)
                .signal_semaphores(&signal_semaphores);

            self.device.queue_submit(self.queue, &[submit_info], frame.fence).unwrap();

            let swapchains = &[self.swapchain];
            let image_indices = &[image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            match self.swapchain_loader.queue_present(self.queue, &present_info) {
                Ok(_) => {}
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
                | Err(vk::Result::SUBOPTIMAL_KHR) => {
                    self.create_swapchain(window, self.swapchain);
                }
                Err(e) => panic!("{:?}", e),
            }

            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

            self.cleanup_old();
        }
    }
}