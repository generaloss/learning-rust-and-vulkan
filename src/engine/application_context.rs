// 'context.rs'

use std::sync::Arc;
use std::collections::HashMap;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use winit::window::{Icon, Window, WindowAttributes};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::dpi::{LogicalSize};
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::window::WindowId;
use crate::engine::vulkan_context::VulkanContext;


pub trait AppAdapter {

    fn init(&mut self, vulkan: &mut VulkanContext);
    fn render(&mut self, vulkan: &mut VulkanContext, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>);
    fn resize(&mut self, width: u32, height: u32);
    fn shutdown(&mut self);

}


struct ContextConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub icon_path: Option<String>,
}

pub struct ContextBuilder {
    config: ContextConfig
}

impl ContextBuilder {

    pub fn new() -> Self {
        Self {
            config: ContextConfig {
                title: "Window".into(),
                width: 800,
                height: 600,
                icon_path: None,
            }
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn icon(mut self, icon_path: &str) -> Self {
        self.config.icon_path = Some(icon_path.into());
        self
    }

    pub fn create(self) -> Context {
        let size = LogicalSize::new(self.config.width, self.config.height);

        let mut window_attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(size)
            .with_name("blazing_fast_engine", self.config.title)
            .with_visible(false);

        if let Some(icon_path) = self.config.icon_path {
            let icon = icon_from_path(icon_path);
            window_attributes = window_attributes.with_window_icon(Some(icon));
        }

        Context::new(window_attributes)
    }

}

fn icon_from_path(icon_path: String) -> Icon {
    let image = image::open(icon_path)
        .expect("Failed to open icon file")
        .to_rgba8();

    let (width, height) = image.dimensions();
    let data = image.into_raw();

    let icon = Icon::from_rgba(data, width, height)
        .expect("Failed to create icon");

    icon
}


pub struct Context {
    window_attributes: WindowAttributes,
    pub window: Option<Arc<Window>>,
    pub app: Option<Box<dyn AppAdapter>>,
    pub vulkan: Option<VulkanContext>
}

impl Context {

    pub fn new(window_attributes: WindowAttributes) -> Context {
        Self {
            window_attributes,
            window: None,
            app: None,
            vulkan: None
        }
    }

    pub fn set_app <A:AppAdapter+'static> (&mut self, app: A) {
        self.app = Some(Box::new(app));
    }

    pub fn clear_app(&mut self) {
        self.app = None;
    }

    pub fn init(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let attributes = self.window_attributes.clone();
        let window = Arc::new(event_loop.create_window(attributes).unwrap());

        // Create vulkan context
        let vulkan = VulkanContext::new(window.clone());
        self.vulkan = Some(vulkan);
        self.window = Some(window);

        // Init application
        if let (Some(app), Some(vulkan)) = (self.app.as_mut(), self.vulkan.as_mut()) {
            app.init(vulkan);
        }
    }

    pub fn render(&mut self) {
        if let (Some(window), Some(vulkan), Some(app)) = (self.window.as_mut(), self.vulkan.as_mut(), self.app.as_mut()) {
            vulkan.render(window, app);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width < 1 || height < 1 {
            return;
        }

        if let Some(vulkan) = self.vulkan.as_mut() {
            vulkan.resize_event(width, height);
        }

        if let Some(app) = self.app.as_mut() {
            app.resize(width, height);
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(app) = self.app.as_mut() {
            app.shutdown();
        }
    }

}


pub struct ContextManager {
    contexts: HashMap<WindowId, Context>,
    pending: Vec<Context>
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
            pending: Vec::new()
        }
    }

    pub fn register(&mut self, context: Context) {
        self.pending.push(context);
    }

    pub fn run(&mut self) -> Result<(), EventLoopError> {
        let event_loop = EventLoop::new()?;
        event_loop.run_app(self)
    }
}

impl ApplicationHandler for ContextManager {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        for mut context in self.pending.drain(..) {
            context.init(event_loop);

            if let Some(window) = context.window.as_mut() {
                self.contexts.insert(window.id(), context);
            }
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let Some(context) = self.contexts.get_mut(&window_id) {
            match event {
                WindowEvent::Resized(size) => {
                    context.resize(size.width, size.height);
                }
                WindowEvent::RedrawRequested => {
                    context.render();
                }
                WindowEvent::CloseRequested => {
                    context.shutdown();
                    self.contexts.remove(&window_id);
                    if self.contexts.is_empty() {
                        _event_loop.exit();
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        for context in self.contexts.values_mut() {
            if let Some(window) = context.window.as_mut() {
                // Отправляем окну запрос на перерисовку. ОС обработает его на следующем шаге своего композитора.
                window.request_redraw();
            }
        }
    }
}