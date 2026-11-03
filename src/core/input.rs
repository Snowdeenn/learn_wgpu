use std::hash::Hash;

use std::collections::HashSet;

use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Debug, Clone)]
struct ButtonState<T> {
    held: HashSet<T>,
    just_pressed: HashSet<T>,
}

impl<T> Default for ButtonState<T> {
    fn default() -> Self {
        Self {
            held: HashSet::new(),
            just_pressed: HashSet::new(),
        }
    }
}

impl<T> ButtonState<T>
where
    T: Copy + Eq + Hash,
{
    fn press(&mut self, button: T) -> bool {
        if !self.held.insert(button) {
            return false;
        }

        self.just_pressed.insert(button);
        true
    }

    fn release(&mut self, button: T) {
        self.held.remove(&button);
    }

    fn is_pressed(&self, button: T) -> bool {
        self.held.contains(&button)
    }

    fn is_just_pressed(&self, button: T) -> bool {
        self.just_pressed.contains(&button)
    }

    fn end_frame(&mut self) {
        self.just_pressed.clear();
    }
}

#[derive(Debug, Clone, Default)]
pub struct Input {
    keyboard: ButtonState<KeyCode>,
    mouse: ButtonState<MouseButton>,
    mouse_position: (f32, f32),
}

impl Input {
    pub fn new() -> Input {
        Default::default()
    }

    pub fn pressed(&mut self, key_code: KeyCode) {
        self.keyboard.press(key_code);
    }

    pub fn released(&mut self, key_code: KeyCode) {
        self.keyboard.release(key_code);
    }

    pub fn is_pressed(&self, key_code: KeyCode) -> bool {
        self.keyboard.is_pressed(key_code)
    }

    pub fn is_just_pressed(&self, key_code: KeyCode) -> bool {
        self.keyboard.is_just_pressed(key_code)
    }

    pub fn mouse_pressed(&mut self, button: MouseButton) {
        if self.mouse.press(button) && button == MouseButton::Left {}
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        self.mouse.release(button);
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse.is_pressed(button)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse.is_just_pressed(button)
    }

    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        !self.mouse.is_pressed(button)
    }

    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn end_frame(&mut self) {
        self.keyboard.end_frame();
        self.mouse.end_frame();
    }
}
