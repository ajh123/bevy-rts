use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod game;

use game::GamePlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.60, 0.80, 0.95)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 30.0,
            affects_lightmapped_meshes: false,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(GamePlugin {
            terrain_config: terrain::TerrainConfig {
                seed: 12345,
                chunk_size: 32,
                tile_size: 2.0,
                view_distance_chunks: 8,
                chunk_spawn_budget_per_frame: 32,
                noise_base_frequency: 0.02,
                noise_octaves: 4,
                noise_persistence: 0.5,
                height_scale: 8.0,
            },
        })
        .run();
}
