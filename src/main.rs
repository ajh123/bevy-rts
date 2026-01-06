use bevy::prelude::*;

mod camera;
mod terrain;
mod terrain_renderer;
mod tile_types;
mod selection;
mod object_system;
mod object_renderer;

#[derive(Resource, Clone)]
pub(crate) struct TerrainConfigRes(pub(crate) terrain::TerrainConfig);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.60, 0.80, 0.95)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 30.0,
            affects_lightmapped_meshes: false,
        })
        .insert_resource(TerrainConfigRes(terrain::TerrainConfig {
            seed: 12345,
            chunk_size: 32,
            tile_size: 4.0,
            view_distance_chunks: 8,
            chunk_spawn_budget_per_frame: 32,
            noise_base_frequency: 0.02,
            noise_octaves: 4,
            noise_persistence: 0.5,
            height_scale: 8.0,
        }))
        .insert_resource(camera::TopDownCameraSettings::default())
        .insert_resource(selection::CursorHitRes::default())
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (
                camera::setup_viewer,
                terrain_renderer::setup_terrain_renderer
                ,
                object_system::setup_object_world,
                object_system::setup_object_types,
                object_renderer::setup_object_renderer
           ),
        )
        .add_systems(
            Update,
            (
                camera::top_down_camera_input,
                camera::update_top_down_camera,
                selection::update_cursor_hit,
                object_system::update_interaction_mode,
                object_system::update_placement_rotation,
                object_system::update_hovered_object,
                object_system::handle_build_destroy_click,
                terrain_renderer::stream_chunks,
                object_renderer::sync_object_chunk_roots,
                object_renderer::update_hologram_preview,
                object_renderer::update_object_chunk_visuals,
                object_renderer::draw_hover_highlight,
            )
                .chain(),
        )
        .run();
}
