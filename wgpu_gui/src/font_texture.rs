use crate::model::Vertex;
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU32,
    sync::Arc,
};

use anyhow::*;
use wgpu::util::DeviceExt;

use image::Luma;
use rusttype::gpu_cache::Cache;
use rusttype::{Font, Point, Scale};

pub struct GlyphModel {
    pub width: f32,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

pub struct FontTexture {
    scale_factor: f32,
    font: Font<'static>,
    font_scale: Scale,
    cache: Cache<'static>,
    image_buffer: image::GrayImage,
    default_glyph_point: Point<f32>,
    glyph_map: HashMap<char, Arc<GlyphModel>>,

    size: wgpu::Extent3d,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl FontTexture {
    pub fn new(scale_factor: f32, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let label = Some("font texture");

        let (cache_width, cache_height) =
            ((1024.0 * scale_factor) as u32, (1024.0 * scale_factor) as u32);
        let cache: Cache<'static> = Cache::builder()
            .dimensions(cache_width, cache_height)
            .build();
        let image_buffer = image::GrayImage::new(cache_width, cache_height);

        // let font_data = include_bytes!("font/GenJyuuGothic-Monospace-Normal.ttf");
        let font_data = include_bytes!("font/HackGenConsole-Regular.ttf");
        let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
        let font_scale = rusttype::Scale::uniform(128.0 * scale_factor);

        let default_glyph_point = rusttype::point(0.0, 0.0);

        let size = wgpu::Extent3d {
            width: cache_width,
            height: cache_height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            &image_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(1 * cache_width), // モノクロ256階調なので1byte
                rows_per_image: NonZeroU32::new(cache_height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Ok(Self {
            scale_factor,
            font,
            font_scale,
            cache,
            image_buffer,
            default_glyph_point,
            glyph_map: HashMap::new(),
            size,
            texture,
            view,
            sampler,
        })
    }

    pub fn chars_on_cache(&self, chars: &Vec<char>) -> bool {
        let uniq: HashSet<&char> = chars.iter().collect();
        uniq.into_iter().all(|c| self.glyph_map.contains_key(c))
    }

    pub fn add_chars(&mut self, chars: Vec<char>, queue: &wgpu::Queue, device: &wgpu::Device) {
        let chars: HashSet<char> = chars.into_iter().collect();

        let uniq_positioned_glyph = chars.into_iter().fold(HashMap::new(), |mut map, c| {
            let glyph = self.font.glyph(c);
            let positioned_glyph = glyph
                .scaled(self.font_scale)
                .positioned(self.default_glyph_point);
            map.insert(c, positioned_glyph);
            map
        });

        uniq_positioned_glyph
            .iter()
            .for_each(|(_, v)| self.cache.queue_glyph(0, v.clone()));

        let image_buffer = &mut self.image_buffer;
        self.cache
            .cache_queued(|rect, data| {
                let mut index = 0;
                for y in rect.min.y..rect.max.y {
                    for x in rect.min.x..rect.max.x {
                        image_buffer.put_pixel(x, y, Luma([data[index]]));
                        index += 1
                    }
                }
            })
            .unwrap();

        // テクスチャの更新
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            &image_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(1 * self.size.width), // モノクロ256階調なので1byte
                rows_per_image: NonZeroU32::new(self.size.height),
            },
            self.size,
        );

        let glyph_map = uniq_positioned_glyph
            .iter()
            .fold(HashMap::new(), |mut map, (c, glyph)| {
                if let Ok(Some((tex_rect, vertex_rect))) = self.cache.rect_for(0, glyph) {
                    let x = self.font_scale.x;
                    let y = self.font_scale.y;

                    let rect = vec![
                        Vertex::new(
                            [
                                vertex_rect.min.x as f32 / x,
                                -vertex_rect.min.y as f32 / y,
                                0.0,
                            ],
                            [tex_rect.min.x, tex_rect.min.y],
                        ), // 左上
                        Vertex::new(
                            [
                                vertex_rect.min.x as f32 / x,
                                -vertex_rect.max.y as f32 / y,
                                0.0,
                            ],
                            [tex_rect.min.x, tex_rect.max.y],
                        ), // 左下
                        Vertex::new(
                            [
                                vertex_rect.max.x as f32 / x,
                                -vertex_rect.min.y as f32 / y,
                                0.0,
                            ],
                            [tex_rect.max.x, tex_rect.min.y],
                        ), // 右上
                        Vertex::new(
                            [
                                vertex_rect.max.x as f32 / x,
                                -vertex_rect.max.y as f32 / y,
                                0.0,
                            ],
                            [tex_rect.max.x, tex_rect.max.y],
                        ), // 右下
                    ];
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(format!("Vertex Buffer {}", c).as_str()),
                            contents: bytemuck::cast_slice(&rect),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                    let indices: Vec<u16> = vec![0, 2, 1, 1, 2, 3];
                    let index_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                    let width =
                        (vertex_rect.min.x - vertex_rect.max.x).abs() as f32 / self.font_scale.x;
                    let glyph = GlyphModel {
                        width,
                        vertex_buffer,
                        index_buffer,
                    };
                    map.insert(*c, Arc::new(glyph));
                }
                map
            });
        self.glyph_map = glyph_map;
    }

    pub fn get_glyph(&self, c: char) -> Option<Arc<GlyphModel>> {
        self.glyph_map.get(&c).map(|g| g.clone())
    }
}
