use bevy::{
    prelude::{Entity, Query, ResMut, Resource, Vec4},
    render::{Extract, sync_world::RenderEntity},
};

use crate::VectorText;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ExtractedVectorText {
    pub(crate) entity: Entity,
    pub(crate) text: String,
    pub(crate) font_size: f32,
    pub(crate) color: Vec4,
}

#[derive(Default, Resource)]
pub(crate) struct ExtractedVectorTexts {
    pub(crate) values: Vec<ExtractedVectorText>,
}

#[derive(Default, Resource)]
pub(crate) struct PreparedVectorTexts {
    pub(crate) values: Vec<ExtractedVectorText>,
}

pub(crate) fn extract_vector_texts(
    mut extracted: ResMut<ExtractedVectorTexts>,
    query: Extract<Query<(RenderEntity, &VectorText)>>,
) {
    extracted.values.clear();
    extracted
        .values
        .extend(query.iter().map(|(entity, text)| ExtractedVectorText {
            entity,
            text: text.text.clone(),
            font_size: text.font_size,
            color: text.color,
        }));
}

pub(crate) fn prepare_vector_texts(
    extracted: bevy::ecs::system::Res<ExtractedVectorTexts>,
    mut prepared: ResMut<PreparedVectorTexts>,
) {
    prepared.values.clear();
    prepared.values.extend(extracted.values.iter().cloned());
}
