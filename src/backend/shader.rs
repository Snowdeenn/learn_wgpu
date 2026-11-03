use super::context;

pub fn load_shader(label: Option<&str>, path: &str) -> Result<wgpu::ShaderModule, std::io::Error> {
    let source = std::fs::read_to_string(path)?;
    let desc = wgpu::ShaderModuleDescriptor {
        label: label,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    };

    Ok(context::device().create_shader_module(desc))
}
