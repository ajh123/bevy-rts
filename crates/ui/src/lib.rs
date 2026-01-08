pub mod toolbar;

pub use toolbar::{
    ToolId, ToolbarActionText, ToolbarRegistry, ToolbarState, ToolbarTool, UiInputCapture,
    bottom_toolbar_system, update_toolbar_state_from_hotkeys,
};

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ToolbarRegistry>()
            .init_resource::<ToolbarActionText>()
            .insert_resource(ToolbarState::default())
            .insert_resource(UiInputCapture::default())
            .add_systems(Update, update_toolbar_state_from_hotkeys)
            .add_systems(EguiPrimaryContextPass, bottom_toolbar_system);
    }
}
