pub mod bind_group;
pub mod buffer;
pub mod context;
pub mod pass;
pub mod pipeline;
pub mod renderer;
pub mod sampler;
pub mod texture;

mod shader;
pub mod text;
pub mod shape;
pub mod sprite;

pub use buffer::write_buffer;
pub use pipeline::{PipelineContext, bind};
pub use shader::load_shader;
pub use sprite::{RenderInstance, Sprite, SpriteInstance, batch, draw, draw_all};
pub use texture::{
    build_atlas_region, load_texture, load_texture_array, register_atlas, register_texture,
    register_texture_array,
};

#[macro_export]
macro_rules! create_marker {
    ($($name:ident),* $(,)?) => {
        $(
            pub struct $name;
        )*
    };
}
