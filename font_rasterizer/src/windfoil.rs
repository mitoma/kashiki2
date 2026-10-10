use crate::{
    errors::FontRasterizerError,
    vector_vertex::{QuadraticCurve as Curve, VectorVertex},
};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Header {
    bounds: [f32; 4],
    band_count: u32,
    band_height: f32,
    inverse_height: f32,
    padding: u32,
    raw_start: u32,
    raw_count: u32,
    raw_padding: [u32; 2],
}

fn bands(pieces: &[Curve]) -> (Header, Vec<Curve>, Vec<[u32; 2]>) {
    let mut bounds = [f32::MAX, f32::MAX, -f32::MAX, -f32::MAX];
    for piece in pieces {
        for point in piece.points() {
            for axis in 0..2 {
                bounds[axis] = bounds[axis].min(point[axis]);
                bounds[axis + 2] = bounds[axis + 2].max(point[axis]);
            }
        }
    }
    if pieces.is_empty() {
        bounds = [0.0; 4];
    }
    let band_count = pieces.len().div_ceil(10).clamp(1, 64) as u32;
    let height = bounds[3] - bounds[1];
    let inverse_height = if height > 0.0 {
        band_count as f32 / height
    } else {
        0.0
    };
    let band_height = if inverse_height > 0.0 {
        inverse_height.recip()
    } else {
        0.0
    };
    let mut buckets = vec![Vec::new(); band_count as usize];
    for piece in pieces {
        let first = piece.start[1].min(piece.end[1]);
        let last = piece.start[1].max(piece.end[1]);
        if first == last {
            continue;
        }
        let band_index = |value: f32| {
            (((value - bounds[1]) * inverse_height).floor().max(0.0) as usize)
                .min(band_count as usize - 1)
        };
        for bucket in &mut buckets[band_index(first)..=band_index(last)] {
            bucket.push(*piece);
        }
    }
    let mut atlas = Vec::new();
    let mut rows = Vec::new();
    for mut bucket in buckets {
        bucket.sort_by(|first, second| {
            let maximum = |curve: &Curve| curve.start[0].max(curve.end[0]);
            maximum(second).total_cmp(&maximum(first))
        });
        rows.push([atlas.len() as u32, bucket.len() as u32]);
        atlas.extend(bucket);
    }
    (
        Header {
            bounds,
            band_count,
            band_height,
            inverse_height,
            padding: 0,
            raw_start: 0,
            raw_count: 0,
            raw_padding: [0; 2],
        },
        atlas,
        rows,
    )
}

pub(crate) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Windfoil paths layout"),
        entries: &std::array::from_fn::<_, 3, _>(|binding| wgpu::BindGroupLayoutEntry {
            binding: binding as u32,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: if binding == 2 {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage { read_only: true }
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }),
    })
}

pub(crate) fn upload(
    device: &wgpu::Device,
    vector: &VectorVertex,
) -> Result<wgpu::BindGroup, FontRasterizerError> {
    let raw = vector.curves();
    let mut pieces = Vec::new();
    for curve in raw {
        curve.monotone(&mut pieces);
    }
    let (mut header, mut atlas, rows) = bands(&pieces);
    header.raw_start = atlas.len() as u32;
    header.raw_count = raw.len() as u32;
    atlas.extend_from_slice(raw);
    if atlas.is_empty() {
        atlas.push(Curve {
            start: [0.0; 2],
            control: [0.0; 2],
            end: [0.0; 2],
        });
    }
    let limit = device
        .limits()
        .max_storage_buffer_binding_size
        .min(device.limits().max_buffer_size);
    for bytes in [
        std::mem::size_of_val(atlas.as_slice()),
        std::mem::size_of_val(rows.as_slice()),
    ] {
        if bytes as u64 > limit {
            return Err(FontRasterizerError::PathBufferTooLarge {
                bytes: bytes as u64,
                limit,
            });
        }
    }
    let create = |label, contents, usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage,
        })
    };
    let curves = create(
        "Windfoil curves",
        bytemuck::cast_slice(&atlas),
        wgpu::BufferUsages::STORAGE,
    );
    let rows = create(
        "Windfoil rows",
        bytemuck::cast_slice(&rows),
        wgpu::BufferUsages::STORAGE,
    );
    let header = create(
        "Windfoil header",
        bytemuck::bytes_of(&header),
        wgpu::BufferUsages::UNIFORM,
    );
    Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Windfoil paths"),
        layout: &layout(device),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: curves.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: rows.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: header.as_entire_binding(),
            },
        ],
    }))
}

impl Curve {
    fn split(self, parameter: f32) -> (Self, Self) {
        let interpolate = |start: [f32; 2], end: [f32; 2]| {
            std::array::from_fn(|axis| start[axis] + (end[axis] - start[axis]) * parameter)
        };
        let first = interpolate(self.start, self.control);
        let second = interpolate(self.control, self.end);
        let middle = interpolate(first, second);
        (
            Self {
                start: self.start,
                control: first,
                end: middle,
            },
            Self {
                start: middle,
                control: second,
                end: self.end,
            },
        )
    }

    fn monotone(self, output: &mut Vec<Self>) {
        let mut extrema = Vec::new();
        for axis in 0..2 {
            let [start, control, end] = self.points().map(|point| f64::from(point[axis]));
            let denominator = start - 2.0 * control + end;
            if denominator != 0.0 {
                let parameter = (start - control) / denominator;
                if parameter > 0.0 && parameter < 1.0 {
                    extrema.push(parameter);
                }
            }
        }
        extrema.sort_by(f64::total_cmp);
        extrema.dedup();
        let mut remaining = self;
        let mut previous = 0.0;
        for parameter in extrema {
            let (piece, rest) = remaining.split(((parameter - previous) / (1.0 - previous)) as f32);
            output.push(piece);
            remaining = rest;
            previous = parameter;
        }
        output.push(remaining);
    }
}

#[cfg(test)]
fn curves(vector: &VectorVertex) -> Vec<Curve> {
    let mut pieces = Vec::new();
    for curve in vector.curves() {
        curve.monotone(&mut pieces);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VectorVertexBuilder;

    #[test]
    fn windfoil_preserves_closed_rectangle() {
        let mut builder = VectorVertexBuilder::new();
        builder.move_to(1.0, 2.0);
        builder.line_to(3.0, 2.0);
        builder.line_to(3.0, 4.0);
        builder.line_to(1.0, 4.0);
        builder.close();
        let pieces = curves(&builder.build());
        assert_eq!(pieces.len(), 4);
        for index in 0..pieces.len() {
            assert_eq!(pieces[index].end, pieces[(index + 1) % pieces.len()].start);
        }
    }

    #[test]
    fn windfoil_splits_both_extrema_and_preserves_endpoints() {
        let curve = Curve {
            start: [0.0, 0.0],
            control: [2.0, 3.0],
            end: [1.0, 1.0],
        };
        assert_eq!(std::mem::size_of::<Curve>(), 24);
        assert_eq!(
            bytemuck::cast_slice::<Curve, f32>(&[curve]),
            &[0.0, 0.0, 2.0, 3.0, 1.0, 1.0]
        );
        let mut pieces = Vec::new();
        curve.monotone(&mut pieces);
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].start, curve.start);
        assert_eq!(pieces[2].end, curve.end);
        for piece in &pieces {
            for axis in 0..2 {
                let [start, control, end] = piece.points().map(|point| point[axis]);
                assert!(control >= start.min(end) - 1e-6);
                assert!(control <= start.max(end) + 1e-6);
            }
        }
        assert_eq!(pieces[0].end, pieces[1].start);
        assert_eq!(pieces[1].end, pieces[2].start);
    }
}
