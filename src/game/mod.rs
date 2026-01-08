pub mod camera;
pub mod input;
pub mod lighting;
pub mod modes;
pub mod physics;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use objects as objects_crate;
use terrain as terrain_crate;
use ui as ui_crate;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(camera::TopDownCameraSettings::default())
            .insert_resource(objects_crate::system::CursorHitRes::default())
            .insert_resource(ui_crate::UiInputCaptureRes::default())
            .insert_resource(ui_crate::ToolbarState::default())
            .init_resource::<ui_crate::ToolbarRegistry>()
            .init_resource::<ui_crate::ToolbarActionText>()
            .insert_resource(terrain_crate::TerrainViewerWorldXzRes::default())
            .add_plugins(DefaultPlugins)
            .add_plugins(EguiPlugin::default())
            .add_plugins(modes::construction::ConstructionModePlugin)
            .add_plugins(modes::destruction::DestructionModePlugin)
            .add_systems(
                Startup,
                (
                    camera::setup_viewer,
                    lighting::setup_sun_light,
                    terrain_crate::render::setup_terrain_renderer,
                    objects_crate::system::setup_object_types,
                    objects_crate::system::setup_object_hovered,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    input::update_ui_input_capture,
                    camera::top_down_camera_input,
                    camera::update_top_down_camera,
                    input::update_cursor_hit,
                    input::update_terrain_viewer_world_xz,
                    ui_crate::update_toolbar_state_from_hotkeys,
                    objects_crate::system::update_hovered_object,
                    terrain_crate::render::stream_chunks,
                )
                    .chain(),
            )
            .add_systems(EguiPrimaryContextPass, ui_crate::bottom_toolbar_system);
    }
}
