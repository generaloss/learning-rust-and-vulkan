// 'input.rs'

use std::collections::HashSet;
pub use winit::keyboard::KeyCode;
use winit::event::ElementState;

pub struct Input {
    down_keys: HashSet<KeyCode>,
    pressed_keys: HashSet<KeyCode>,
    up_keys: HashSet<KeyCode>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            down_keys: HashSet::new(),
            pressed_keys: HashSet::new(),
            up_keys: HashSet::new(),
        }
    }

    pub fn handle_key_event(&mut self, key_code: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.pressed_keys.insert(key_code) {
                    self.down_keys.insert(key_code);
                }
            }
            ElementState::Released => {
                if self.pressed_keys.remove(&key_code) {
                    self.up_keys.insert(key_code);
                }
            }
        }
    }

    pub fn clear_frame_states(&mut self) {
        self.down_keys.clear();
        self.up_keys.clear();
    }

    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        self.down_keys.contains(&key_code)
    }

    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.pressed_keys.contains(&key_code)
    }

    pub fn is_key_up(&self, key_code: KeyCode) -> bool {
        self.up_keys.contains(&key_code)
    }
}