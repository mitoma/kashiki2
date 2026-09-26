use bevy::prelude::*;
use bevy_vector_text::{VectorText, VectorTextFont, VectorTextPlugin};
use font_collector::{FontCollector, FontRepository};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VectorTextPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let mut collector = FontCollector::default();
    collector.add_system_fonts();
    let mut repository = FontRepository::new(collector);
    repository.set_primary_font("BIZ UDゴシック");
    /*
    if let Some(font_name) = repository.list_font_names().first().cloned() {
        repository.set_primary_font(&font_name);
    }
     */
    commands.insert_resource(VectorTextFont::from_repository(&repository));
    commands.spawn(
        VectorText::new(
            "炊紙 / Bevy vector text",
            64.0,
            Vec4::new(0.95, 0.9, 0.75, 1.0),
        )
        .with_position(Vec2::new(0.0, 0.0)),
    );
}
