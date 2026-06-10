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
use crate::engine::input::Input;
use crate::engine::vulkan_context::VulkanContext;


pub trait AppAdapter {

    fn init(&mut self, context: &mut ContextFields);
    fn render(&mut self, context: &mut ContextFields, builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>);
    fn resize(&mut self, context: &mut ContextFields, width: u32, height: u32);
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


pub struct ContextFields {
    pub window: Arc<Window>,
    pub vulkan: VulkanContext,
    pub input: Input,
    pub should_close: bool,
}


pub struct Context {
    window_attributes: WindowAttributes,
    app: Option<Box<dyn AppAdapter>>,
    fields: Option<ContextFields>,
}

impl Context {

    pub fn new(window_attributes: WindowAttributes) -> Context {
        Self {
            window_attributes,
            app: None,
            fields: None,
        }
    }

    pub fn set_app <A:AppAdapter+'static> (&mut self, app: A) {
        self.app = Some(Box::new(app));
    }

    pub fn clear_app(&mut self) {
        self.app = None;
    }

    pub fn init(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = self.window_attributes.clone();

        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        let vulkan = VulkanContext::new(window.clone());
        let input = Input::new();

        self.fields = Some(
            ContextFields {
                window,
                vulkan,
                input,
                should_close: false,
            }
        );

        if let (Some(app), Some(fields)) = (self.app.as_mut(), self.fields.as_mut()) {
            app.init(fields);
        }
    }

    pub fn render(&mut self) {
        if let Some(fields) = self.fields.as_mut() {
            if let Some(app) = self.app.as_mut() {

                let window_size = fields.window.inner_size();

                if let Some((mut builder, frame_info)) = fields.vulkan.begin_frame(window_size) {
                    app.render(fields, &mut builder);
                    fields.window.pre_present_notify();
                    fields.vulkan.end_frame(builder, frame_info, window_size);
                }
            }
            fields.input.clear_frame_states();
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width < 1 || height < 1 {
            return;
        }

        if let Some(fields) = self.fields.as_mut() {
            fields.vulkan.resize_event(width, height);

            if let Some(app) = self.app.as_mut() {
                app.resize(fields, width, height);
            }
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

            if let Some(fields) = context.fields.as_mut() {
                self.contexts.insert(fields.window.id(), context);
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

                    if let Some(fields) = context.fields.as_mut() {
                        if fields.should_close {
                            context.shutdown();
                            self.contexts.remove(&window_id);
                            if self.contexts.is_empty() {
                                _event_loop.exit();
                            }
                            return;
                        }
                    }
                }
                WindowEvent::CloseRequested => {
                    context.shutdown();
                    self.contexts.remove(&window_id);
                    if self.contexts.is_empty() {
                        _event_loop.exit();
                    }
                    return;
                }
                WindowEvent::KeyboardInput { device_id: _, event: key_event, is_synthetic: _ } => {
                    if let Some(fields) = context.fields.as_mut() {
                        if let winit::keyboard::PhysicalKey::Code(key_code) = key_event.physical_key {
                            fields.input.handle_key_event(key_code, key_event.state);
                        }
                    }
                }
                _ => {}
            }
        }

    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        for context in self.contexts.values_mut() {
            if let Some(fields) = context.fields.as_mut() {
                // Отправляем окну запрос на перерисовку. ОС обработает его на следующем шаге своего композитора.
                fields.window.request_redraw();
            }
        }
    }

}