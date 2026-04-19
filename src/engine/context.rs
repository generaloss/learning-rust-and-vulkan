use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};
use crate::engine::app_adapter::AppAdapter;
use crate::engine::vulkan_context::VulkanContext;

pub struct Context {
    window_attributes: WindowAttributes,
    pub window: Option<Window>,
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
        // create window
        let attributes = self.window_attributes.clone();
        let window = event_loop.create_window(attributes).unwrap();

        // create vulkan context
        let vulkan = VulkanContext::new(&window);
        self.vulkan = Some(vulkan);
        self.window = Some(window);

        // init()
        if let Some(app) = self.app.as_mut() {
            app.init();
        }
    }

    pub fn render(&mut self) {
        if let Some(window) = self.window.as_mut() {
            if let Some(vulkan) = self.vulkan.as_mut() {
                vulkan.render(window);
            }
        }


        if let Some(app) = self.app.as_mut() {
            app.render();
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width < 1 || height < 1 {
            return;
        }

        if let Some(vulkan) = self.vulkan.as_mut() {
            vulkan.resize_event();
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