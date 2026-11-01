mod manager;
mod ui;
mod widget;

pub use manager::DebugManager;
pub use ui::DebugUi;
pub use widget::WidgetBuilder;

use crate::app::resource::Resources;
use crate::app::{
    CircleMesh, Fps, IndexBuffer, IndexCount, InstanceBuffer, InstanceData, RectMesh,
    ScenePipeline, TriangleMesh, VertexBuffer,
};
use crate::backend;

pub fn run(dm: &DebugManager, rs: &Resources) {
    let fps = rs.read_resource::<Fps>();

    dm.create_widget("Debug Panel")
        .window()
        .size([360.0, 500.0])
        .show(|w| {
            w.metric("FPS", &fps);

            let mut selected = *rs.read_resource::<ScenePipeline>();
            w.create_widget("Sélection de la pipeline")
                .header(true)
                .show(|w| {
                    w.ui.radio_value(&mut selected, ScenePipeline::Mesh, "Mesh Pipeline");
                    if w.ui
                        .radio_value(&mut selected, ScenePipeline::Texture, "Texture Pipeline")
                        .changed()
                    {
                        let mesh = rs.read_resource::<RectMesh>();
                        let vertices = rs.read_resource::<VertexBuffer>();
                        let indices = rs.read_resource::<IndexBuffer>();
                        backend::write_buffer(vertices.as_raw(), 0, mesh.vertices_bytes());
                        backend::write_buffer(indices.as_raw(), 0, mesh.indices_bytes());
                        **rs.write_resource::<IndexCount>() = mesh.indices().len() as u32;
                    }
                });
            *rs.write_resource::<ScenePipeline>() = selected;

            if selected == ScenePipeline::Mesh {
                w.create_widget("Sélection de la forme")
                    .header(true)
                    .show(|w| {
                        if w.button("Triangle").clicked() {
                            let vbuffer = rs.read_resource::<VertexBuffer>();
                            let ibuffer = rs.read_resource::<IndexBuffer>();
                            let t_mesh = rs.read_resource::<TriangleMesh>();
                            let mut icount = rs.write_resource::<IndexCount>();
                            **icount = t_mesh.indices().len() as u32;
                            backend::write_buffer(vbuffer.as_raw(), 0, t_mesh.vertices_bytes());
                            backend::write_buffer(ibuffer.as_raw(), 0, t_mesh.indices_bytes());
                        }
                        if w.button("Rect").clicked() {
                            let vbuffer = rs.read_resource::<VertexBuffer>();
                            let ibuffer = rs.read_resource::<IndexBuffer>();
                            let r_mesh = rs.read_resource::<RectMesh>();
                            let mut icount = rs.write_resource::<IndexCount>();
                            **icount = r_mesh.indices().len() as u32;
                            backend::write_buffer(vbuffer.as_raw(), 0, r_mesh.vertices_bytes());
                            backend::write_buffer(ibuffer.as_raw(), 0, r_mesh.indices_bytes());
                        }
                        if w.button("Circle").clicked() {
                            let vbuffer = rs.read_resource::<VertexBuffer>();
                            let ibuffer = rs.read_resource::<IndexBuffer>();
                            let c_mesh = rs.read_resource::<CircleMesh>();
                            let mut icount = rs.write_resource::<IndexCount>();
                            **icount = c_mesh.indices().len() as u32;
                            backend::write_buffer(vbuffer.as_raw(), 0, c_mesh.vertices_bytes());
                            backend::write_buffer(ibuffer.as_raw(), 0, c_mesh.indices_bytes());
                        }

                    });
            }

            w.create_widget("Paramètres des instances")
                .header(true)
                .show(|w| {
                    let mut instances = rs.write_resource::<InstanceData>();
                    let mut changed = false;

                    for (index, instance) in instances.iter_mut().enumerate() {
                        w.ui.push_id(index, |ui| {
                            let mut w = DebugUi::new(ui);
                            w.create_widget(format!("Instance {index}"))
                                .header(true)
                                .show(|w| {
                                    changed |= w
                                        .slider("Offset x", &mut instance.offset[0], -300.0..=300.0)
                                        .changed();
                                    changed |= w
                                        .slider("Offset y", &mut instance.offset[1], -300.0..=300.0)
                                        .changed();
                                    changed |= w
                                        .slider("Scale x", &mut instance.scale[0], 0.0..=200.0)
                                        .changed();
                                    changed |= w
                                        .slider("Scale y", &mut instance.scale[1], 0.0..=200.0)
                                        .changed();
                                });
                        });
                    }

                    if changed {
                        // let buffer = rs.read_resource::<InstanceBuffer>();
                        // backend::write_buffer(
                        //     buffer.as_raw(),
                        //     0,
                        //     bytemuck::cast_slice(instances.as_slice()),
                        // );
                    }
                });
        });
}
