use std::ops::{Deref, DerefMut};

use math::{Mat4, Vec2};

use winit::keyboard::KeyCode;

use crate::core::Input;

pub struct Camera2D {
    pos: Vec2,
    zoom: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Camera2D {
            pos: Vec2::zero(),
            zoom: 1.0,
        }
    }
    pub fn get_view_proj(&self, screen_w: f32, screen_h: f32) -> Mat4 {
        let screen_w = screen_w / self.zoom;
        let screen_h = screen_h / self.zoom;
        let proj = Mat4::orthographic_wgpu(0.0, screen_w, screen_h, 0.0, -1.0, 1.0);
        let view = Mat4::translation(
            -self.pos.x + (screen_w * 0.5),
            -self.pos.y + (screen_h * 0.5),
            0.0,
        );
        proj.multiply(view)
    }
    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.pos = Vec2::new(x, y);
    }
    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Physical pixels per world unit. Panics for nonpositive or nonfinite values.
    #[allow(dead_code, reason = "API du moteur conservee pour les autres scenes et modules.")]
    pub fn set_zoom(&mut self, zoom: f32) {
        assert!(
            zoom.is_finite() && zoom > 0.0,
            "Camera zoom must be finite and positive"
        );
        self.zoom = zoom;
    }

    pub fn world_to_screen(&self, position: [f32; 2], resolution: [u32; 2]) -> [f32; 2] {
        [
            (position[0] - self.pos.x) * self.zoom + resolution[0] as f32 * 0.5,
            (position[1] - self.pos.y) * self.zoom + resolution[1] as f32 * 0.5,
        ]
    }
}

pub struct Camera2DController {
    speed: f32,
    state: Input,
}

impl Camera2DController {
    /// Vitesse de deplacement en unites du monde par seconde.
    pub fn new(speed: f32) -> Self {
        Camera2DController {
            speed,
            state: Input::new(),
        }
    }

    pub fn update(&self, cam: &mut Camera2D, dt: f32) {
        let speed = self.speed;

        let move_dir = {
            let mut dir = [0.0f32, 0.0f32];
            if self.is_pressed(KeyCode::KeyD) {
                dir[0] += 1.0;
            }
            if self.is_pressed(KeyCode::KeyA) {
                dir[0] -= 1.0;
            }
            if self.is_pressed(KeyCode::KeyS) {
                dir[1] += 1.0;
            }
            if self.is_pressed(KeyCode::KeyW) {
                dir[1] -= 1.0;
            }

            let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
            if len > 0.0 {
                [dir[0] / len, dir[1] / len]
            } else {
                dir
            }
        };

        let former_pos = cam.pos();
        let x = former_pos.x + move_dir[0] * speed * dt;
        let y = former_pos.y + move_dir[1] * speed * dt;

        cam.set_pos(x, y);
    }
}

impl Deref for Camera2DController {
    type Target = Input;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for Camera2DController {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
