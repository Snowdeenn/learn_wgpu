use super::resources::{CircleMesh, Fps, InstanceData, RectMesh, ScenePipeline, TriangleMesh};
use crate::core::Resources;
use crate::debug::{DebugManager, DebugUi};

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
                    w.ui.radio_value(&mut selected, ScenePipeline::Texture, "Texture Pipeline");
                });
            *rs.write_resource::<ScenePipeline>() = selected;

            if selected == ScenePipeline::Mesh {
                w.create_widget("Sélection de la forme")
                    .header(true)
                    .show(|w| {
                        if w.button("Triangle").clicked() {
                            let mesh = **rs.read_resource::<TriangleMesh>();
                            for instance in rs.write_resource::<InstanceData>().iter_mut() {
                                instance.mesh = mesh;
                            }
                        }
                        if w.button("Rect").clicked() {
                            let mesh = **rs.read_resource::<RectMesh>();
                            for instance in rs.write_resource::<InstanceData>().iter_mut() {
                                instance.mesh = mesh;
                            }
                        }
                        if w.button("Circle").clicked() {
                            let mesh = **rs.read_resource::<CircleMesh>();
                            for instance in rs.write_resource::<InstanceData>().iter_mut() {
                                instance.mesh = mesh;
                            }
                        }
                    });
            }

            w.create_widget("Paramètres des instances")
                .header(true)
                .show(|w| {
                    let mut instances = rs.write_resource::<InstanceData>();

                    for (index, instance) in instances.iter_mut().enumerate() {
                        w.ui.push_id(index, |ui| {
                            let mut w = DebugUi::new(ui);
                            w.create_widget(format!("Instance {index}"))
                                .header(true)
                                .show(|w| {
                                    w.slider("Offset x", &mut instance.offset[0], -300.0..=300.0);
                                    w.slider("Offset y", &mut instance.offset[1], -300.0..=300.0);
                                    w.slider("Scale x", &mut instance.scale[0], 0.0..=200.0);
                                    w.slider("Scale y", &mut instance.scale[1], 0.0..=200.0);
                                });
                        });
                    }
                });
        });
}
