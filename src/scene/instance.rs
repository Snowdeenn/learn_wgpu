use crate::backend::{self, Sprite};

pub struct Dummy {
    pub mesh: new_collection::Id<backend::shape::MeshResource>,
    pub color: [f32; 4],
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub sprite: backend::Sprite,
}

impl backend::shape::ShapeInstance for Dummy {
    fn mesh(&self) -> new_collection::Id<backend::shape::MeshResource> {
        self.mesh
    }

    fn render_instance(&self) -> backend::RenderInstance {
        backend::SpriteInstance::render_instance(self)
    }
}

impl backend::SpriteInstance for Dummy {
    fn render_instance(&self) -> backend::RenderInstance {
        let layer = match &self.sprite {
            Sprite::Array { layer, .. } => *layer,
            Sprite::Animated { current_layer, .. } => *current_layer,
            _ => 0,
        };
        backend::RenderInstance {
            color: self.color,
            offset: self.offset,
            scale: self.scale,
            layer,
            _padding: [0; 3],
            uv_min: [0.0; 2],
            uv_max: [1.0; 2],
        }
    }

    fn sprite(&self) -> &backend::Sprite {
        &self.sprite
    }
}
