use winit::dpi::{LogicalSize};
use winit::window::Window;
use crate::engine::context::Context;

pub struct ContextConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
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
                height: 600
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

    pub fn create(self) -> Context {
        let size = LogicalSize::new(self.config.width, self.config.height);

        let window_attributes = Window::default_attributes()
            .with_title(self.config.title)
            .with_inner_size(size)
            .with_visible(false);

        Context::new(window_attributes)
    }

}