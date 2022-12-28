use anyhow::Context;
use ttf_parser::{Face, OutlineBuilder, Rect};

const FONT_DATA: &[u8] = include_bytes!("../../wgpu_gui/src/font/HackGenConsole-Regular.ttf");

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct FontVertex {
    pub(crate) position: [f32; 3],
    pub(crate) wait: [f32; 3],
}

struct InternalFontVertex {
    x: f32,
    y: f32,
    wait: (f32, f32),
}

impl FontVertex {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FontVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

struct FontVertexBuilder {
    main_vertex: Vec<InternalFontVertex>,
    main_index: Vec<u16>,
    current_main_index: u16,
    vertex_swap: bool,
}

impl FontVertexBuilder {
    fn new() -> Self {
        FontVertexBuilder {
            main_vertex: vec![InternalFontVertex {
                x: 0.0,
                y: 0.0,
                wait: (0.0, 0.0),
            }],
            main_index: Vec::new(),
            current_main_index: 0,
            vertex_swap: false,
        }
    }

    #[inline]
    fn next_wait(&mut self) -> (f32, f32) {
        self.vertex_swap = !self.vertex_swap;
        if self.vertex_swap {
            (0.0, 1.0)
        } else {
            (0.0, 0.0)
        }
    }

    fn build(&mut self, rect: Rect) -> (Vec<FontVertex>, Vec<u16>) {
        let mut is_first = true;
        let vertex = self
            .main_vertex
            .iter()
            .map(|InternalFontVertex { x, y, wait }| {
                let (x, y) = if is_first {
                    is_first = false;
                    (0.0, 0.0)
                } else {
                    (
                        (*x / rect.width() as f32) - 0.5,
                        (*y / rect.height() as f32) - 0.5,
                    )
                };
                println!("x:{}, y:{}, wait:{:?}", x, y, wait);
                FontVertex {
                    position: [x, y, 0.0],
                    wait: [1.0, wait.0, wait.1],
                }
            })
            .collect();
        println!("index:{:?}", &self.main_index);
        (vertex, self.main_index.clone())
    }
}

impl OutlineBuilder for FontVertexBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.main_vertex.push(InternalFontVertex { x, y, wait });
        self.current_main_index = self.main_vertex.len() as u16 - 1;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let wait = self.next_wait();
        self.main_vertex.push(InternalFontVertex { x, y, wait });
        self.main_index.push(0);
        self.main_index.push(self.current_main_index);
        self.current_main_index = self.main_vertex.len() as u16 - 1;
        self.main_index.push(self.current_main_index);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let wait = self.next_wait();
        let pre_index = self.current_main_index;
        self.main_vertex.push(InternalFontVertex { x, y, wait });
        let post_index = self.main_vertex.len() as u16 - 1;
        self.current_main_index = post_index;

        self.main_index.push(0);
        self.main_index.push(pre_index);
        self.main_index.push(post_index);

        // ベジエ曲線
        self.main_vertex.push(InternalFontVertex {
            x: x1,
            y: y1,
            wait: (1.0, 0.0),
        });
        let control_index = self.main_vertex.len() as u16 - 1;
        self.main_index.push(pre_index);
        self.main_index.push(control_index);
        self.main_index.push(post_index);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        todo!()
    }

    fn close(&mut self) {
        //
    }
}

impl FontVertex {
    pub(crate) fn new_char(c: char) -> anyhow::Result<(Vec<FontVertex>, Vec<u16>)> {
        let face = Face::parse(FONT_DATA, 0)?;
        let glyph_id = face.glyph_index(c).with_context(|| "get glyph for face")?;
        let mut builder = FontVertexBuilder::new();
        let rect = face
            .outline_glyph(glyph_id, &mut builder)
            .with_context(|| "create char vertex")?;
        Ok(builder.build(rect))
    }
}
