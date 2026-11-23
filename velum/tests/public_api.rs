use velum::backend::pass::{PassError, RenderOperation};
use velum::{RenderData, Scene, SceneError, Velum, scene_renderer};

#[test]
fn draw_sort_metadata_supports_default_depth_and_custom_order() {
    use new_collection::Id;
    use velum::backend::{
        DrawSortKey, RenderInstance,
        instances::Instance,
        shape::{MeshResource, ShapeInstance},
    };

    struct DefaultObject;
    impl Instance for DefaultObject {}
    impl ShapeInstance for DefaultObject {
        fn mesh(&self) -> Id<MeshResource> {
            panic!("Sorting does not resolve the mesh")
        }
        fn render_instance(&self) -> RenderInstance {
            RenderInstance {
                depth: 0.25,
                ..bytemuck::Zeroable::zeroed()
            }
        }
    }
    assert_eq!(
        DefaultObject.sort_key(),
        DrawSortKey {
            draw_order: 0,
            depth: 0.25
        }
    );

    struct OrderedObject;
    impl Instance for OrderedObject {}
    impl ShapeInstance for OrderedObject {
        fn mesh(&self) -> Id<MeshResource> {
            panic!("Not needed")
        }
        fn render_instance(&self) -> RenderInstance {
            panic!("Custom keys bypass GPU conversion")
        }
        fn sort_key(&self) -> DrawSortKey {
            DrawSortKey {
                draw_order: 10,
                depth: 0.75,
            }
        }
    }
    assert_eq!(
        OrderedObject.sort_key(),
        DrawSortKey {
            draw_order: 10,
            depth: 0.75
        }
    );
}

#[test]
fn global_resource_handles_support_ownership() {
    use velum::backend::{binding_set, buffer, pipeline, shape, target, text, texture};
    use velum::core::ResourceHandle;
    fn supports<H: ResourceHandle>() {}
    supports::<texture::TextureId>();
    supports::<new_collection::Id<shape::MeshResource>>();
    supports::<target::RenderTargetId>();
    supports::<binding_set::TextureBindingSetId>();
    supports::<text::TextId>();
    supports::<pipeline::PipelineId<u32>>();
    supports::<buffer::Buffer<[f32; 4], buffer::Uniform>>();
}

fn empty_scene() -> Scene {
    Scene {
        renderer: scene_renderer::renderer().build(),
        data: RenderData::new(),
    }
}

#[test]
fn scene_selection_is_validated_and_preserves_state() {
    let mut velum = Velum::new();
    let world = velum.add_scene(empty_scene());
    let menu = velum.add_scene(empty_scene());
    let value = velum
        .scene_mut(world)
        .unwrap()
        .data
        .insert(String::from("world"));
    velum.activate(world).unwrap();
    velum.remove_scene(menu).unwrap();
    assert!(matches!(
        velum.activate(menu),
        Err(SceneError::MissingScene(_))
    ));
    assert_eq!(velum.active_scene(), Some(world));
    assert_eq!(
        velum.scene(world).unwrap().data.require(value).unwrap(),
        "world"
    );
    velum.deactivate();
    assert_eq!(velum.active_scene(), None);
    velum.activate(world).unwrap();
    velum.remove_scene(world).unwrap();
    assert_eq!(velum.active_scene(), None);
    assert!(velum.remove_scene(world).is_none());
}

#[test]
fn typed_inputs_are_independent_and_missing_inputs_are_diagnostic() {
    let mut data = RenderData::new();
    let first = data.insert(String::from("first"));
    let second = data.insert(String::from("second"));
    data.require_mut(first).unwrap().push('!');
    assert_eq!(data.require(second).unwrap(), "second");
    assert_eq!(data.remove(first).unwrap(), "first!");
    assert!(data.remove(first).is_none());
    let replacement = data.insert(String::from("replacement"));
    let error = data.require(first).unwrap_err();
    assert!(error.expected_type.contains("String"));
    assert!(!error.handle.is_empty());
    let error: PassError = error.into();
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(data.require(replacement).unwrap(), "replacement");
}

// Compile the custom-operation and pass API as an external consumer.
struct CustomOperation;
impl RenderOperation for CustomOperation {
    fn execute(
        &mut self,
        _: &mut wgpu::RenderPass<'_>,
        _: &mut RenderData,
    ) -> Result<(), PassError> {
        Ok(())
    }
}

#[test]
fn renderer_builder_accepts_backend_passes() {
    let mut builder = scene_renderer::renderer();
    let pass = builder.add_pass(velum::backend::pass::render(
        velum::backend::pass::PassTarget::Surface,
        wgpu::LoadOp::Load,
        CustomOperation,
        true,
    ));
    let mut renderer = builder.build();
    renderer.deactivate_pass(pass);
    renderer.activate_pass(pass);
}
