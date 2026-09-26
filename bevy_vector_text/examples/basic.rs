use bevy::prelude::*;
use bevy_vector_text::{VectorText, VectorTextPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VectorTextPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(VectorText::new(
        "炊紙 / Bevy vector text",
        48.0,
        Vec4::new(0.95, 0.9, 0.75, 1.0),
    ));
}
