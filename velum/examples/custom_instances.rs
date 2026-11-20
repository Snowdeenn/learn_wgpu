//! Application CPU objects and a custom GPU format, without `RenderInstance`.
use velum::backend::{
    Geometry, GeometryId, InstanceBinding, InstanceSource, MaterialId, MaterialTemplate,
    ObjectInstance, PhaseId, SortPolicy, bindings, context, geometry,
    instances::{self, Instance},
    object::{self, RenderPhase},
    pass,
    renderer::OffscreenFrame,
    try_shader,
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Payload {
    transform: [f32; 4],
    color: [f32; 4],
}
struct Body {
    geometry: GeometryId,
    material: MaterialId,
    payload: Payload,
}
impl Instance for Body {}
impl InstanceSource for Body {
    type Gpu = Payload;
    fn gpu_data(&self) -> Payload {
        self.payload
    }
    fn binding() -> InstanceBinding {
        InstanceBinding::Storage {
            group: 3,
            binding: 7,
        }
    }
}
impl ObjectInstance for Body {
    fn geometry(&self) -> GeometryId {
        self.geometry
    }
    fn material(&self) -> MaterialId {
        self.material
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    context::init(&adapter)?;
    let shader = try_shader(
        None,
        "application payload",
        r#"
struct Instance { transform:vec4f, color:vec4f }
@group(3) @binding(7) var<storage,read> objects:array<Instance>;
struct Out { @builtin(position) p:vec4f, @location(0) color:vec4f }
@vertex fn vs_main(@builtin(vertex_index) v:u32,@builtin(instance_index) i:u32)->Out {
    var positions=array<vec2f,3>(vec2f(-1.,-1.),vec2f(1.,-1.),vec2f(0.,1.));
    let object=objects[i];
    return Out(vec4f(positions[v]*object.transform.zw+object.transform.xy,0.,1.),object.color);
}
@fragment fn fs_main(input:Out)->@location(0) vec4f {return input.color;}
"#,
    )?;
    let mut description = MaterialTemplate::fullscreen(
        None,
        "application payload",
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    description.schema = velum::binding_schema! {3 => [bindings::buffer_entry(7,wgpu::ShaderStages::VERTEX,wgpu::BufferBindingType::Storage {read_only:true},32,false)]};
    let template = velum::try_material_template!(description)?.share();
    let material = velum::try_material! {template:template,bindings:[]}?;
    let geometry = geometry::try_register(Geometry::generated(None, 0..3))?;
    let objects = [
        Body {
            geometry: geometry.id(),
            material: material.id(),
            payload: Payload {
                transform: [-0.45, 0., 0.4, 0.7],
                color: [1., 0.2, 0.1, 1.],
            },
        },
        Body {
            geometry: geometry.id(),
            material: material.id(),
            payload: Payload {
                transform: [0.45, 0., 0.4, 0.7],
                color: [0.1, 0.8, 0.4, 1.],
            },
        },
    ];
    let output = context::device(None).create_texture(&wgpu::TextureDescriptor {
        label: Some("custom instance output"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut frame = OffscreenFrame::new(None, output.create_view(&Default::default()))?;
    let mut phase = RenderPhase::new(PhaseId::OPAQUE, SortPolicy::PreserveOrder);
    instances::submit(None, &objects, &mut phase)?;
    let mut data = velum::RenderData::new();
    let phase = data.insert(phase);
    let mut renderer = velum::SceneRenderer::new();
    renderer.add_pass(pass::render(
        pass::PassTarget::Surface,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        object::PhaseOperation { phase },
        true,
    ));
    frame.execute(&mut renderer, &mut data)?;
    let report = frame.finish();
    assert_eq!((report.draw_calls, report.instances), (1, 2));
    println!(
        "{} draw, {} instances applicatives, {} octets transférés",
        report.draw_calls, report.instances, report.uploaded_bytes
    );
    Ok(())
}
