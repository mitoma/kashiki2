use bevy::{
    prelude::*,
    render::{
        ExtractSchedule, Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
    },
};

mod render;

/// Text rendered by the vector text renderer.
#[derive(Component, Clone, Debug, ExtractComponent, Reflect)]
#[reflect(Component)]
pub struct VectorText {
    pub text: String,
    pub font_size: f32,
    pub color: Vec4,
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
        app.register_type::<VectorText>()
            .add_plugins(ExtractComponentPlugin::<VectorText>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<render::ExtractedVectorTexts>()
                .init_resource::<render::PreparedVectorTexts>()
                .add_systems(ExtractSchedule, render::extract_vector_texts)
                .add_systems(
                    Render,
                    render::prepare_vector_texts.in_set(RenderSystems::Prepare),
                );
        }
    }
}
