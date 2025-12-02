use bezier_converter::CubicBezier;
use log::debug;
use rustybuzz::ttf_parser::OutlineBuilder;

pub(crate) struct VectorVertexBuilder {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    path_start_index: Option<u32>,
    vertex_swap: FlipFlop,
    builder_options: VertexBuilderOptions,
}

#[allow(dead_code)]
impl VectorVertexBuilder {
    pub fn new() -> Self {
        Self {
            vertex: Vec::new(),
            index: Vec::new(),
            // index 0 は原点B、1 は原点L に予約されているので、2 から開始する
            current_index: 1,
            path_start_index: None,
            vertex_swap: FlipFlop::Flip,
            builder_options: VertexBuilderOptions::default(),
        }
    }

    pub fn with_options(self, builder_options: VertexBuilderOptions) -> Self {
        Self {
            vertex: self.vertex,
            index: self.index,
            current_index: self.current_index,
            path_start_index: self.path_start_index,
            vertex_swap: self.vertex_swap,
            builder_options,
        }
    }

    #[inline]
    fn next_wait(&mut self) -> FlipFlop {
        self.vertex_swap = self.vertex_swap.next();
        self.vertex_swap
    }

    pub fn build(self) -> VectorVertex {
        let center: [f32; 2] = self.builder_options.center;
        let unit_em: f32 = self.builder_options.unit_em;
        let coordinate_system = self.builder_options.coordinate_system;
        let scale_option = self.builder_options.scale;
        let [center_x, center_y] = coordinate_system.transform(center[0], center[1]);
        let [center_x, center_y] = scale_option.map_or([center_x, center_y], |[width, height]| {
            [center_x * width, center_y * height]
        });

        let vertex = self
            .vertex
            .iter()
            .map(|InternalVertex { x, y, wait }| {
                let [x, y] = coordinate_system.transform(*x, *y);
                let [x, y] = [(x - center_x) / unit_em, (y - center_y) / unit_em];
                let [x, y] = scale_option.map_or([x, y], |[width, height]| [x * width, y * height]);
                Vertex {
                    position: [x, y],
                    vertex_type: wait.vertex_type(),
                }
            })
            .collect();
        VectorVertex {
            vertex,
            index: self.index,
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.vertex.push(InternalVertex { x, y, wait });
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });
        self.path_start_index = Some(self.current_index);
        self.current_index += 2;
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        let Some(last) = &self.vertex.last() else {
            return;
        };
        if last.x == x && last.y == y {
            // 同じ座標への line_to は無視する
            return;
        }

        let wait = self.next_wait();
        self.vertex.push(InternalVertex { x, y, wait });
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });
        self.index.push(1); // 原点L の index
        self.index.push(self.current_index);
        self.index.push(self.current_index + 2);
        self.current_index += 2;
    }

    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let wait = self.next_wait();

        self.vertex.push(InternalVertex {
            x: x1,
            y: y1,
            wait: FlipFlop::Control,
        });
        self.vertex.push(InternalVertex { x, y, wait });
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });

        self.index.push(0); // 原点B の index
        self.index.push(self.current_index - 1);
        self.index.push(self.current_index + 2);

        // ベジエ曲線
        self.index.push(self.current_index - 1);
        self.index.push(self.current_index + 1);
        self.index.push(self.current_index + 2);
        self.current_index += 3;
    }

    pub fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // 3 次ベジエを 2 次ベジエに近似する
        let last = &self.vertex.last().unwrap();
        let cb = CubicBezier {
            x0: last.x,
            y0: last.y,
            x1: x,
            y1: y,
            cx0: x1,
            cy0: y1,
            cx1: x2,
            cy1: y2,
        };
        let qbs = cb.to_quadratic();
        debug!("cubic to quadratic: 1 -> {}", qbs.len());
        for qb in qbs.iter() {
            self.quad_to(qb.cx0, qb.cy0, qb.x1, qb.y1)
        }
    }

    pub fn close(&mut self) {
        if let Some(start_index) = self.path_start_index {
            let start_vertex = &self.vertex[(start_index) as usize];
            self.line_to(start_vertex.x, start_vertex.y);
            self.path_start_index = None;
        }
    }
}

impl OutlineBuilder for VectorVertexBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.curve_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.close();
    }
}

pub enum CoordinateSystem {
    Svg,  // SVGの座標系 (左上原点, Y軸が下方向)
    Font, // フォント座標系 (ベースライン原点, Y軸が上方向)
}

impl CoordinateSystem {
    #[inline]
    pub(crate) fn transform(&self, x: f32, y: f32) -> [f32; 2] {
        match self {
            CoordinateSystem::Svg => [x, -y],
            CoordinateSystem::Font => [x, y],
        }
    }
}

pub(crate) struct VertexBuilderOptions {
    pub(crate) center: [f32; 2],
    pub(crate) unit_em: f32,
    pub(crate) coordinate_system: CoordinateSystem,
    pub(crate) scale: Option<[f32; 2]>,
}

impl Default for VertexBuilderOptions {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            unit_em: 1.0,
            coordinate_system: CoordinateSystem::Font,
            scale: None,
        }
    }
}

impl VertexBuilderOptions {
    pub fn new(
        center: [f32; 2],
        unit_em: f32,
        coordinate_system: CoordinateSystem,
        scale: Option<[f32; 2]>,
    ) -> Self {
        Self {
            center,
            unit_em,
            coordinate_system,
            scale,
        }
    }
}

#[derive(Debug)]
pub struct VectorVertex {
    pub(crate) vertex: Vec<Vertex>,
    pub(crate) index: Vec<u32>,
}

impl VectorVertex {
    pub fn vertex_size(&self) -> u64 {
        (self.vertex.len() * std::mem::size_of::<Vertex>()) as u64
    }

    pub fn index_size(&self) -> u64 {
        (self.index.len() * std::mem::size_of::<u32>()) as u64
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 2],
    pub(crate) vertex_type: u32,
}

impl Vertex {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // 文字情報なので xy の座標だけでよい
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FlipFlop {
    Flip,
    Flop,
    Control,
    FlipForLine,
    FlopForLine,
}

impl FlipFlop {
    #[inline]
    pub(crate) fn next(&self) -> Self {
        match self {
            FlipFlop::Flip => FlipFlop::Flop,
            FlipFlop::Flop => FlipFlop::Flip,
            FlipFlop::Control => FlipFlop::Control,
            FlipFlop::FlipForLine => FlipFlop::FlipForLine,
            FlipFlop::FlopForLine => FlipFlop::FlopForLine,
        }
    }

    pub(crate) fn for_line(&self) -> Self {
        match self {
            FlipFlop::Flip => FlipFlop::FlopForLine,
            FlipFlop::Flop => FlipFlop::FlipForLine,
            FlipFlop::Control => FlipFlop::Control,
            FlipFlop::FlipForLine => FlipFlop::FlipForLine,
            FlipFlop::FlopForLine => FlipFlop::FlopForLine,
        }
    }

    #[inline]
    pub(crate) fn vertex_type(&self) -> u32 {
        match self {
            FlipFlop::Flip => 2,
            FlipFlop::FlipForLine => 3,
            FlipFlop::Flop => 4,
            FlipFlop::FlopForLine => 5,
            FlipFlop::Control => 6,
        }
    }
}

pub(crate) struct InternalVertex {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) wait: FlipFlop,
}
