use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;
use crate::engine::context::Context;

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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if let Some(context) = self.contexts.get_mut(&window_id) {
            match event {
                WindowEvent::CloseRequested => {
                    context.shutdown();
                    self.contexts.remove(&window_id);
                    // exit
                    if self.contexts.is_empty() {
                        event_loop.exit();
                    }
                }
                WindowEvent::Resized(size) => {
                    context.resize(size.width, size.height);
                }
                WindowEvent::RedrawRequested => {
                    context.render();
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        for context in self.contexts.values_mut() {
            if let Some(window) = context.window.as_mut() {
                window.request_redraw();
            }
        }
    }

}