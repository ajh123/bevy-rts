use bevy::prelude::*;

mod camera;
mod terrain;
mod terrain_renderer;

#[derive(Resource, Clone)]
pub(crate) struct TerrainConfigRes(pub(crate) terrain::TerrainConfig);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.60, 0.80, 0.95)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 300.0,
            affects_lightmapped_meshes: false,
        })
        .insert_resource(TerrainConfigRes(terrain::TerrainConfig {
            seed: 12345,
            chunk_size: 32,
            tile_size: 2.0,
            view_distance_chunks: 8,
            chunk_spawn_budget_per_frame: 32,
            noise_base_frequency: 0.02,
            noise_octaves: 4,
            noise_persistence: 0.5,
            height_scale: 8.0,
        }))
        .insert_resource(camera::TopDownCameraSettings::default())
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (camera::setup_viewer, terrain_renderer::setup_terrain_renderer))
        .add_systems(
            Update,
            (
                camera::top_down_camera_input,
                camera::update_top_down_camera.after(camera::top_down_camera_input),
                terrain_renderer::stream_chunks.after(camera::top_down_camera_input),
            ),
        )
        .run();
}
