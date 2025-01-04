use bezier_converter::CubicBezier;
use log::debug;

pub(crate) struct VectorVertexBuilder {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    vertex_swap: FlipFlop,
    builder_options: BuilderOptions,
}

impl VectorVertexBuilder {
    pub fn new() -> Self {
        Self {
            vertex: Vec::new(),
            index: Vec::new(),
            current_index: 0,
            vertex_swap: FlipFlop::Flip,
            builder_options: BuilderOptions::default(),
        }
    }

    pub fn with_options(self, builder_options: BuilderOptions) -> Self {
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
        let vertex = self
            .vertex
            .iter()
            .map(|InternalVertex { x, y, wait }| {
                let x = (*x - center[0]) / unit_em;
                let y = (*y - center[1]) / unit_em;
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
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: self.vertex_swap.next(),
        });
        self.index.push(self.current_index);
        self.current_index += 1;
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.vertex.push(InternalVertex { x, y, wait });
        self.index.push(0);
        self.index.push(self.current_index);
        self.index.push(self.current_index + 1);
        self.current_index += 1;
    }

    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let wait = self.next_wait();

        self.vertex.push(InternalVertex {
            x: x1,
            y: y1,
            wait: FlipFlop::Control,
        });
        self.vertex.push(InternalVertex { x, y, wait });

        self.index.push(0);
        self.index.push(self.current_index);
        self.index.push(self.current_index + 2);

        // ベジエ曲線
        self.index.push(self.current_index);
        self.index.push(self.current_index + 1);
        self.index.push(self.current_index + 2);
        self.current_index += 2;
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

pub(crate) struct BuilderOptions {
    center: [f32; 2],
    unit_em: f32,
}

impl Default for BuilderOptions {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            unit_em: 1.0,
        }
    }
}

impl BuilderOptions {
    pub fn new(center: [f32; 2], unit_em: f32) -> Self {
        Self { center, unit_em }
    }
}

pub(crate) struct VectorVertex {
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
    pub(crate) wait: [f32; 2],
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
    fn wait(&self) -> [f32; 2] {
        match self {
            FlipFlop::Flip => [0.0, 0.0],
            FlipFlop::Flop => [0.0, 1.0],
            FlipFlop::Control => [1.0, 0.0],
        }
    }
}

struct InternalVertex {
    x: f32,
    y: f32,
    wait: FlipFlop,
}
