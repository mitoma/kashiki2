use bezier_converter::CubicBezier;
use log::debug;
use rustybuzz::ttf_parser::OutlineBuilder;

pub(crate) struct VectorVertexBuilder {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    vertex_swap: FlipFlop,
    builder_options: VertexBuilderOptions,
}

impl VectorVertexBuilder {
    pub fn new() -> Self {
        Self {
            vertex: Vec::new(),
            index: Vec::new(),
            current_index: 0,
            vertex_swap: FlipFlop::Flip,
            builder_options: VertexBuilderOptions::default(),
        }
    }

    pub fn with_options(self, builder_options: VertexBuilderOptions) -> Self {
        Self {
            vertex: self.vertex,
            index: self.index,
            current_index: self.current_index,
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
                    wait: wait.wait(),
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
        self.current_index += 1;
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();

        let [center_x, center_y]: [f32; 2] = self.builder_options.center;

        // 原点, 始点, 終点の三角形用の頂点を登録
        let current = self.vertex.last().unwrap();
        self.vertex.push(InternalVertex {
            x: current.x * 2.0 - center_x,
            y: current.y * 2.0 - center_y,
            wait: current.wait,
        });
        self.vertex.push(InternalVertex {
            x: x * 2.0 - center_x,
            y: y * 2.0 - center_y,
            wait,
        });

        self.vertex.push(InternalVertex { x, y, wait });
        self.index.push(0);
        self.index.push(self.current_index + 1);
        self.index.push(self.current_index + 2);
        self.current_index += 3;
    }

    /*
    // 定性的に 200.0 を閾値にしている
    const BEZIER_THRESHOLD: f32 = 200.0;
    // ベジエ曲線が直線に近似できるかどうかを判定する
    fn is_narrow_bezier(x1: f32, y1: f32, cx: f32, cy: f32, x2: f32, y2: f32) -> bool {
        let line_vec = (x2 - x1, y2 - y1);
        let d1 = (cx - x1) * line_vec.1 - (cy - y1) * line_vec.0;
        let d2 = (x2 - cx) * line_vec.1 - (y2 - cy) * line_vec.0;
        d1.abs() < Self::BEZIER_THRESHOLD && d2.abs() < Self::BEZIER_THRESHOLD
    }
     */

    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        /* ベジエ曲線の直線への近似はあまり効果がない感じがするので一旦無くす
        let current = self.vertex.last().unwrap();
        // (current.x, current.y) と (x, y) でなす線分上に (x1, y1) が一定の誤差内で存在する時には line_to として処理する
        if Self::is_narrow_bezier(current.x, current.y, x1, y1, x, y) {
            info!(
                "quad_to: (x1, y1) is on the line to (x, y), use line_to instead. cx: {}, cy: {}, x1: {}, y1: {}, x: {}, y: {}",
                current.x, current.y, x1, y1, x, y
            );
            self.line_to(x, y);
            return;
        }
         */

        let wait = self.next_wait();

        // 原点, 始点, 終点の三角形用の頂点を登録
        let current = self.vertex.last().unwrap();
        self.vertex.push(InternalVertex {
            x: current.x,
            y: current.y,
            wait: FlipFlop::Control,
        });
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: FlipFlop::Control,
        });

        // ベジエ曲線用の制御点と終点を登録
        self.vertex.push(InternalVertex {
            x: x1,
            y: y1,
            wait: FlipFlop::Control,
        });
        self.vertex.push(InternalVertex { x, y, wait });

        // 原点, 始点, 終点の三角形
        self.index.push(0);
        self.index.push(self.current_index + 1);
        self.index.push(self.current_index + 2);

        // ベジエ曲線
        self.index.push(self.current_index);
        self.index.push(self.current_index + 3);
        self.index.push(self.current_index + 4);
        self.current_index += 4;
    }

    pub fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // 3 次ベジエを 2 次ベジエに近似する
        let last = &self.vertex[(self.current_index - 1) as usize];
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
        // noop
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
    fn transform(&self, x: f32, y: f32) -> [f32; 2] {
        match self {
            CoordinateSystem::Svg => [x, -y],
            CoordinateSystem::Font => [x, y],
        }
    }
}

pub(crate) struct VertexBuilderOptions {
    center: [f32; 2],
    unit_em: f32,
    coordinate_system: CoordinateSystem,
    scale: Option<[f32; 2]>,
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
    // ベジエ曲線を描くために 3 頂点のうちどれを制御点、どれを始点・終点と区別するかを表す。
    // 典型的には [0, 0], または [0, 1] が始点か終点。[1, 0] 制御点となる。
    pub(crate) wait: [f32; 3],
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
                // ベジエか直線かの情報が必要なので [f32; 2] を使っている。
                // 本質的には 2 bit でいいはずなので調整余地あり
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Clone, Copy)]
enum FlipFlop {
    Flip,
    Flop,
    Control,
}

impl FlipFlop {
    #[inline]
    fn next(&self) -> Self {
        match self {
            FlipFlop::Flip => FlipFlop::Flop,
            FlipFlop::Flop => FlipFlop::Flip,
            FlipFlop::Control => FlipFlop::Control,
        }
    }

    #[inline]
    fn wait(&self) -> [f32; 3] {
        match self {
            FlipFlop::Flip => [1.0, 0.0, 0.0],
            FlipFlop::Flop => [1.0, 0.0, 1.0],
            FlipFlop::Control => [1.0, 1.0, 0.0],
        }
    }
}

struct InternalVertex {
    x: f32,
    y: f32,
    wait: FlipFlop,
}
