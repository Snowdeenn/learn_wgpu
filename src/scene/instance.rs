use crate::backend;

pub struct Dummy {
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub sprite: backend::Sprite,
}

impl backend::SpriteInstance for Dummy {
    fn render_instance(&self) -> backend::RenderInstance {
        backend::RenderInstance {
            color: self.color,
            offset: self.offset,
            scale: self.scale,
        }
    }

    fn sprite(&self) -> &backend::Sprite {
        &self.sprite
    }
}
