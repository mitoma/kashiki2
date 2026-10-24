use std::collections::{HashMap, HashSet};
use wgpu::util::DeviceExt;

pub(crate) const COMPUTE_SOURCE: &str = r#"
fn instance_vector(base: u32) -> vec4<f32> {
    return bitcast<vec4<f32>>(vec4<u32>(instance_words[base], instance_words[base + 1u],
        instance_words[base + 2u], instance_words[base + 3u]));
}

@compute @workgroup_size(64)
fn project_curves(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let instance = invocation.y;
    let point = invocation.x;
    if instance >= projection_settings.x || point >= path_header.raw_count * 3u { return; }
    let base = instance * 23u;
    let attributes = InstancesInput(instance_vector(base), instance_vector(base + 4u),
        instance_vector(base + 8u), instance_vector(base + 12u), instance_vector(base + 16u).xyz,
        instance_words[base + 19u], instance_words[base + 20u], bitcast<f32>(instance_words[base + 21u]),
        instance_words[base + 22u]);
    projected_points[instance * path_header.raw_count * 3u + point] =
        project_local(path_points[path_header.raw_start * 3u + point], attributes);
}
"#;

pub(crate) fn layout(device: &wgpu::Device, compute: bool) -> wgpu::BindGroupLayout {
    let stages = if compute {
        wgpu::ShaderStages::COMPUTE
    } else {
        wgpu::ShaderStages::VERTEX_FRAGMENT
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Projected curves layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: if compute {
                    stages
                } else {
                    wgpu::ShaderStages::NONE
                },
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: stages,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: !compute,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: stages,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

#[derive(Clone)]
pub(crate) struct Projection {
    pub compute: wgpu::BindGroup,
    pub render: wgpu::BindGroup,
    pub enabled: bool,
}

#[derive(Default)]
pub(crate) struct ProjectionCache {
    entries: HashMap<(wgpu::Buffer, u32, u32, bool), Projection>,
    active: HashSet<(wgpu::Buffer, u32, u32, bool)>,
}

impl ProjectionCache {
    pub fn begin_frame(&mut self) {
        self.active.clear();
    }

    pub fn end_frame(&mut self) {
        self.entries.retain(|key, _| self.active.contains(key));
    }

    pub fn get(
        &mut self,
        device: &wgpu::Device,
        source: &wgpu::Buffer,
        curves: u32,
        instances: u32,
        requested: bool,
    ) -> Projection {
        let bytes = u64::from(curves) * u64::from(instances) * 48;
        let limits = device.limits();
        let enabled = requested
            && curves != 0
            && bytes
                <= limits
                    .max_storage_buffer_binding_size
                    .min(limits.max_buffer_size)
            && instances <= limits.max_compute_workgroups_per_dimension
            && (curves * 3).div_ceil(64) <= limits.max_compute_workgroups_per_dimension;
        let key = (source.clone(), curves, instances, enabled);
        self.active.insert(key.clone());
        self.entries
            .entry(key)
            .or_insert_with(|| {
                let output = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Projected control points"),
                    size: if enabled { bytes.max(16) } else { 16 },
                    usage: wgpu::BufferUsages::STORAGE,
                    mapped_at_creation: false,
                });
                let settings = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Curve projection settings"),
                    contents: bytemuck::cast_slice(&[instances, curves, u32::from(enabled), 0]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let create = |compute| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Projected curves"),
                        layout: &layout(device, compute),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: source.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: output.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: settings.as_entire_binding(),
                            },
                        ],
                    })
                };
                Projection {
                    compute: create(true),
                    render: create(false),
                    enabled,
                }
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compute_instance_word_layout_matches_vertex_buffer() {
        use crate::vector_instances::InstanceRaw;
        assert_eq!(std::mem::size_of::<InstanceRaw>(), 23 * 4);
        assert_eq!(std::mem::offset_of!(InstanceRaw, model), 0);
    }
}
