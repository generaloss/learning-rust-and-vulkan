// 'input.rs'

use std::collections::HashSet;
use glam::{DVec2, Vec2};
use winit::dpi::PhysicalPosition;
pub use winit::keyboard::KeyCode;
use winit::event::{ElementState, MouseScrollDelta};
pub use winit::event::MouseButton;

pub struct Input {
    // keyboard
    just_pressed_keys: HashSet<KeyCode>,
    pressed_keys: HashSet<KeyCode>,
    released_keys: HashSet<KeyCode>,
    // mouse
    just_pressed_buttons: HashSet<MouseButton>,
    pressed_buttons: HashSet<MouseButton>,
    released_buttons: HashSet<MouseButton>,

    pub position: DVec2,
    pub delta: DVec2,
    pub scroll_delta: Vec2,
}

impl Input {
    pub fn new() -> Self {
        Self {
            just_pressed_keys: HashSet::new(),
            pressed_keys: HashSet::new(),
            released_keys: HashSet::new(),

            // mouse
            just_pressed_buttons: HashSet::new(),
            pressed_buttons: HashSet::new(),
            released_buttons: HashSet::new(),
            position: DVec2::ZERO,
            delta: DVec2::ZERO,
            scroll_delta: Vec2::ZERO,
        }
    }


    pub fn handle_key_event(&mut self, key_code: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.pressed_keys.insert(key_code) {
                    self.just_pressed_keys.insert(key_code);
                }
            }
            ElementState::Released => {
                if self.pressed_keys.remove(&key_code) {
                    self.released_keys.insert(key_code);
                }
            }
        }
    }

    pub fn handle_mouse_input_event(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.pressed_buttons.insert(button) {
                    self.just_pressed_buttons.insert(button);
                }
            }
            ElementState::Released => {
                if self.pressed_buttons.remove(&button) {
                    self.released_buttons.insert(button);
                }
            }
        }
    }

    pub fn handle_cursor_moved_event(&mut self, position: PhysicalPosition<f64>) {
        self.position.x = position.x;
        self.position.y = position.y;
    }

    pub fn handle_cursor_motion_event(&mut self, delta: (f64, f64)) {
        self.delta.x += delta.0;
        self.delta.y += delta.1;
    }

    pub fn handle_mouse_wheel_event(&mut self, delta: MouseScrollDelta) {
        let (x, y) = match delta {
            MouseScrollDelta::LineDelta(line_x, line_y) => (line_x, line_y),
            MouseScrollDelta::PixelDelta(physical_pos) => {
                let pixels_per_line = 38.0;
                (
                    (physical_pos.x / pixels_per_line) as f32,
                    (physical_pos.y / pixels_per_line) as f32,
                )
            }
        };

        self.scroll_delta.x += x;
        self.scroll_delta.y += y;
    }


    pub fn clear_frame_states(&mut self) {
        self.just_pressed_keys.clear();
        self.released_keys.clear();
        self.just_pressed_buttons.clear();
        self.released_buttons.clear();

        self.delta.x = 0.0;
        self.delta.y = 0.0;
        self.scroll_delta.x = 0.0;
        self.scroll_delta.y = 0.0;
    }


    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key_code)
    }

    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.pressed_keys.contains(&key_code)
    }

    pub fn is_key_up(&self, key_code: KeyCode) -> bool {
        self.released_keys.contains(&key_code)
    }


    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.just_pressed_buttons.contains(&button)
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    pub fn is_button_up(&self, button: MouseButton) -> bool {
        self.released_buttons.contains(&button)
    }
}