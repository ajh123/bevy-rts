use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

mod game;

use game::camera;
use game::input;
use game::modes;
use game::ui::toolbar;
use game::world::objects;
use game::world::terrain;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.60, 0.80, 0.95)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 30.0,
            affects_lightmapped_meshes: false,
        })
        .insert_resource(terrain::types::TerrainConfigRes(
            terrain::types::TerrainConfig {
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
        ))
        .insert_resource(camera::TopDownCameraSettings::default())
        .insert_resource(input::CursorHitRes::default())
        .insert_resource(camera::UiInputCaptureRes::default())
        .insert_resource(toolbar::ToolbarState::default())
        .init_resource::<toolbar::ToolbarRegistry>()
        .init_resource::<toolbar::ToolbarActionText>()
        .init_resource::<objects::render::ObjectEntityMap>()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(modes::construction::ConstructionModePlugin)
        .add_plugins(modes::destruction::DestructionModePlugin)
        .add_systems(
            Startup,
            (
                camera::setup_viewer,
                terrain::render::setup_terrain_renderer,
                objects::system::setup_object_world,
                objects::system::setup_object_types,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                camera::update_ui_input_capture,
                camera::top_down_camera_input,
                camera::update_top_down_camera,
                input::update_cursor_hit,
                toolbar::update_toolbar_state_from_hotkeys,
                objects::system::update_hovered_object,
                terrain::render::stream_chunks,
                objects::render::sync_objects,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, toolbar::bottom_toolbar_system)
        .run();
}
