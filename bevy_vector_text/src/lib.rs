use bevy::{
    core_pipeline::{Core2dSystems, schedule::Core2d},
    prelude::*,
    render::{
        ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
    },
    shader::Shader,
};
use font_collector::{FontData, FontRepository};
use font_rasterizer::{
    VectorVertexData, char_width_calcurator::CharWidthCalculator,
    font_converter::convert_char_to_vector_vertices,
};
use std::sync::Arc;

mod render;

/// Text rendered by the vector text renderer.
#[derive(Component, Clone, Debug, ExtractComponent, Reflect)]
#[reflect(Component)]
pub struct VectorText {
    pub text: String,
    pub font_size: f32,
    pub color: Vec4,
}

#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct VectorTextGeometry {
    pub data: VectorVertexData,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VectorTextFillRule {
    EvenOdd,
    #[default]
    NonZero,
}

#[derive(Resource)]
pub struct VectorTextFont {
    fonts: Arc<Vec<FontData>>,
    ascii_override_font: Option<FontData>,
    char_width_calculator: CharWidthCalculator,
}

impl VectorTextFont {
    pub fn from_repository(repository: &FontRepository) -> Self {
        let fonts = Arc::new(repository.get_fonts());
        Self {
            char_width_calculator: CharWidthCalculator::new(fonts.clone()),
            fonts,
            ascii_override_font: repository.get_ascii_override_font(),
        }
    }
}

impl VectorTextGeometry {
    pub fn new(data: VectorVertexData) -> Self {
        Self { data }
    }
}

impl VectorText {
    pub fn new(text: impl Into<String>, font_size: f32, color: Vec4) -> Self {
        Self {
            text: text.into(),
            font_size,
            color,
        }
    }
}

/// Registers the ECS side of the vector text renderer.
///
/// GPU extraction and render graph integration are added after this data contract
/// is validated by the standalone example.
pub struct VectorTextPlugin;

impl Plugin for VectorTextPlugin {
    fn build(&self, app: &mut App) {
        let shader = app
            .world_mut()
            .resource_mut::<Assets<Shader>>()
            .add(Shader::from_wgsl(render::VECTOR_TEXT_SHADER, file!()));
        app.register_type::<VectorText>()
            .init_resource::<VectorTextFillRule>()
            .add_plugins((
                ExtractComponentPlugin::<VectorText>::default(),
                ExtractComponentPlugin::<VectorTextGeometry>::default(),
            ))
            .add_systems(Update, update_vector_text_geometry);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<render::ExtractedVectorTexts>()
                .init_resource::<render::PreparedVectorTexts>()
                .init_resource::<render::ExtractedVectorGeometries>()
                .init_resource::<render::GpuVectorTextBuffers>()
                .init_resource::<VectorTextFillRule>()
                .insert_resource(render::VectorTextShader(shader))
                .add_systems(ExtractSchedule, render::extract_vector_texts)
                .add_systems(ExtractSchedule, render::extract_vector_geometries)
                .add_systems(RenderStartup, render::init_vector_text_pipeline)
                .add_systems(
                    Render,
                    render::prepare_vector_texts.in_set(RenderSystems::Prepare),
                )
                .add_systems(
                    Render,
                    render::prepare_vector_text_buffers.in_set(RenderSystems::PrepareResources),
                )
                .add_systems(
                    Core2d,
                    render::draw_vector_texts.after(Core2dSystems::MainPass),
                );
        }
    }
}

fn update_vector_text_geometry(
    font: Option<Res<VectorTextFont>>,
    mut commands: Commands,
    query: Query<(Entity, &VectorText), Changed<VectorText>>,
) {
    let Some(font) = font else {
        return;
    };

    for (entity, text) in &query {
        commands
            .entity(entity)
            .insert(VectorTextGeometry::new(build_text_geometry(
                font.as_ref(),
                text,
            )));
    }
}

fn build_text_geometry(font: &VectorTextFont, text: &VectorText) -> VectorVertexData {
    let scale = text.font_size / 256.0;
    let mut advance = -0.9;
    let mut positions = Vec::new();
    let mut vertex_types = Vec::new();
    let mut indices = Vec::new();

    for character in text.text.chars() {
        let width = font.char_width_calculator.get_width(character);
        let Ok((glyph, _)) = convert_char_to_vector_vertices(
            font.fonts.clone(),
            font.ascii_override_font.clone(),
            character,
            width,
        ) else {
            advance += width.to_f32() * scale;
            continue;
        };
        let glyph = glyph.into_data();
        let vertex_offset = positions.len() as u32;

        positions.extend([[0.0, 0.0], [0.0, 0.0]]);
        vertex_types.extend([0, 1]);
        positions.extend(
            glyph
                .positions
                .into_iter()
                .map(|[x, y]| [advance + x * scale, y * scale]),
        );
        vertex_types.extend(glyph.vertex_types);
        indices.extend(glyph.indices.into_iter().map(|index| index + vertex_offset));
        advance += width.to_f32() * scale;
    }

    VectorVertexData {
        positions,
        vertex_types,
        indices,
    }
}
