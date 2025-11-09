use bezier_converter::CubicBezier;
use log::debug;
use rustybuzz::ttf_parser::OutlineBuilder;

use crate::vector_vertex::{FlipFlop, InternalVertex, VectorVertex, Vertex, VertexBuilderOptions};

#[allow(dead_code)]
pub(crate) struct VectorVertexBuilderV0 {
    vertex: Vec<InternalVertex>,
    index: Vec<u32>,
    current_index: u32,
    vertex_swap: FlipFlop,
    pub(crate) builder_options: VertexBuilderOptions,
}

#[allow(dead_code)]
impl VectorVertexBuilderV0 {
    pub fn new() -> Self {
        Self {
            vertex: Vec::new(),
            index: Vec::new(),
            // index 0 は原点B、1 は原点L に予約されているので、2 から開始する
            current_index: 1,
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
        //let [center_x, center_y] = scale_option.map_or([center_x, center_y], |[width, height]| {
        //    [center_x * width, center_y * height]
        //});

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
        self.vertex.push(InternalVertex {
            x,
            y,
            wait: wait.for_line(),
        });
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

impl OutlineBuilder for VectorVertexBuilderV0 {
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
