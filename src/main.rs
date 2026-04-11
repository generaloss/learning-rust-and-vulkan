pub mod engine;

use winit::error::EventLoopError;
use engine::context_builder::ContextBuilder;
use engine::app_adapter::AppAdapter;
use crate::engine::context_manager::ContextManager;

fn main() -> Result<(), EventLoopError> {
    let mut context = ContextBuilder::new()
        .title("Test")
        .size(1280, 720)
        .create();

    context.set_app(Game);

    let mut manager = ContextManager::new();
    manager.register(context);
    manager.run()?;
    Ok(())
}

struct Game;

impl AppAdapter for Game {
    fn init(&mut self) {
        println!("Init");
    }

    fn render(&mut self) {
        println!("Render");
    }

    fn resize(&mut self, width: u32, height: u32) {
        println!("Resize {}x{}", width, height);
    }

    fn shutdown(&mut self) {
        println!("Shutdown");
    }
}

