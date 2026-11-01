pub mod renderer;
pub mod bind_group;
pub mod buffer;
pub mod context;
pub mod sampler;
pub mod texture;
pub mod pipeline;
pub mod pass;
pub mod mesh;

mod helper;
pub use helper::*;

mod pipeline_context;
pub use pipeline_context::*;

mod draw;
pub use draw::*;

#[macro_export]
macro_rules! create_marker {
    ($($name:ident),* $(,)?) => {
        $(
            pub struct $name;
        )*
    };
}

#[macro_export]
macro_rules! unique_resource {
    ($name:ident, $ty:ty) => {
        pub struct $name(pub $ty);

        impl std::ops::Deref for $name {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}
