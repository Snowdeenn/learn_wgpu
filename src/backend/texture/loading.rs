use super::{AtlasRegion, Texture, TextureBuilder, TextureResource};
use crate::backend::{context, sampler};
use image::GenericImageView;
use new_collection::{Arena, Id};

pub fn load_texture(label: &str, path: &str) -> Result<wgpu::Texture, image::ImageError> {
    let image = image::open(path)?;
    let rgba_image = image.to_rgba8();

    let (width, height) = image.dimensions();
    let image_byte = rgba_image.as_raw();

    let dst_tex = TextureBuilder::new()
        .with_label(label)
        .with_size(wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        })
        .build();

    context::queue().write_texture(
        wgpu::TexelCopyTextureInfoBase {
            texture: &dst_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image_byte,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        dst_tex.size(),
    );

    Ok(dst_tex)
}

pub fn load_texture_array(
    label: &str,
    paths: &[&str],
) -> Result<Texture, Box<dyn std::error::Error>> {
    if paths.is_empty() {
        return Err("Une texture array doit contenir au moins une image".into());
    }

    let images = paths
        .iter()
        .map(|path| image::open(path).map(|image| image.to_rgba8()))
        .collect::<Result<Vec<_>, _>>()?;

    let (width, height) = images[0].dimensions();

    if images
        .iter()
        .any(|image| image.dimensions() != (width, height))
    {
        return Err("Les images doivent avoir les mêmes dimensions".into());
    }

    let layers = u32::try_from(images.len())?;
    let limits = context::device().limits();

    if layers > limits.max_texture_array_layers {
        return Err("Nombre de couches supérieur à la limite GPU".into());
    }

    if width > limits.max_texture_dimension_2d || height > limits.max_texture_dimension_2d {
        return Err("Dimensions supérieures à la limite GPU".into());
    }

    let texture = TextureBuilder::new()
        .with_label(label)
        .with_size(wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers,
        })
        .build();

    for (layer, image) in images.iter().enumerate() {
        context::queue().write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            image.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1, // Une couche par écriture.
            },
        );
    }

    Ok(Texture::with_view(
        texture,
        &wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        },
    ))
}
pub fn register_texture(raw: wgpu::Texture) -> (Id<TextureResource>, wgpu::BindGroupLayout) {
    let tex = Texture::from_raw(raw);
    let sampler = sampler::filtering("");
    let (lay, group) = crate::bind_group! {
        wgpu::ShaderStages::VERTEX_FRAGMENT => tex,
        wgpu::ShaderStages::VERTEX_FRAGMENT => sampler
    };
    let resource = TextureResource::Separate(super::Resource {
        texture: tex,
        sampler: sampler.as_raw().clone(),
        bind_group: group,
    });
    let id = {
        let mut storage = context::texture_storage().write().unwrap();
        storage.insert(resource)
    };

    (id, lay)
}

pub fn register_texture_array(
    array: Texture,
) -> Result<(Id<TextureResource>, wgpu::BindGroupLayout), &'static str> {
    let dim = array.as_raw().dimension();
    if !matches!(dim, wgpu::TextureDimension::D2) || array.as_raw().depth_or_array_layers() == 1 {
        return Err("Only array texture can be use with register_texture_array()");
    }

    let sampler = sampler::filtering("");
    let (lay, group) = crate::bind_group! {
        wgpu::ShaderStages::VERTEX_FRAGMENT => array,
        wgpu::ShaderStages::VERTEX_FRAGMENT => sampler
    };
    let resource = super::Resource {
        texture: array,
        sampler: sampler.as_raw().clone(),
        bind_group: group,
    };

    let id = {
        let mut storage = context::texture_storage().write().unwrap();
        storage.insert(TextureResource::Array(resource))
    };

    Ok((id, lay))
}

pub fn register_atlas(
    atlas: wgpu::Texture,
    regions: Vec<AtlasRegion>,
) -> (Id<TextureResource>, Vec<Id<AtlasRegion>>) {
    let tex = Texture::from_raw(atlas);
    let sampler = sampler::filtering("");
    let (_, group) = crate::bind_group! {
        wgpu::ShaderStages::VERTEX_FRAGMENT => tex,
        wgpu::ShaderStages::VERTEX_FRAGMENT => sampler
    };

    let mut atlas_regions = Arena::new();
    let mut region_ids = Vec::new();

    for region in regions {
        let id = atlas_regions.insert(region);
        region_ids.push(id);
    }
    let resource = TextureResource::Atlas {
        resource: super::Resource {
            texture: tex,
            sampler: sampler.as_raw().clone(),
            bind_group: group,
        },
        regions: atlas_regions,
    };
    let id = {
        let mut storage = context::texture_storage().write().unwrap();
        storage.insert(resource)
    };

    (id, region_ids)
}
