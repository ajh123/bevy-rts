pub mod camera;
pub mod input;
pub mod lighting;
pub mod modes;
pub mod physics;

use bevy::prelude::*;

use objects as objects_crate;
use terrain as terrain_crate;
use ui as ui_crate;

pub struct GamePlugin {
    pub terrain_config: terrain_crate::TerrainConfig,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum StartupSet {
    Camera,
    Lighting,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum UpdateSet {
    UiCapture,
    CameraInput,
    CameraUpdate,
    CursorHit,
    TerrainViewer,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ui_crate::UiPlugin)
            .add_plugins(terrain_crate::TerrainPlugin {
                config: self.terrain_config.clone(),
            })
            .add_plugins(objects_crate::ObjectsPlugin)
            .add_plugins(camera::CameraPlugin)
            .add_plugins(input::InputPlugin)
            .add_plugins(lighting::LightingPlugin)
            .add_plugins(modes::construction::ConstructionModePlugin)
            .add_plugins(modes::destruction::DestructionModePlugin)
            .configure_sets(
                Startup,
                (
                    StartupSet::Camera,
                    StartupSet::Lighting.after(StartupSet::Camera),
                ),
            )
            .configure_sets(
                Update,
                (
                    UpdateSet::UiCapture,
                    UpdateSet::CameraInput.after(UpdateSet::UiCapture),
                    UpdateSet::CameraUpdate.after(UpdateSet::CameraInput),
                    UpdateSet::CursorHit.after(UpdateSet::CameraUpdate),
                    UpdateSet::TerrainViewer.after(UpdateSet::CursorHit),
                ),
            );
    }
}
