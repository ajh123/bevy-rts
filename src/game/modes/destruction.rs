use bevy::prelude::*;

use objects::highlight;
use objects::system::{HoveredObject, ObjectKind, ObjectTypes};
use terrain::TerrainWorld;
use ui::{ToolId, ToolbarActionText, ToolbarRegistry, ToolbarState, ToolbarTool, UiInputCapture};

pub struct DestructionModePlugin;

impl Plugin for DestructionModePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_destruction_toolbar)
            .add_systems(
                Update,
                (
                    draw_hover_highlight,
                    handle_destruction_click,
                    draw_destruction_ui,
                ),
            );
    }
}

fn setup_destruction_toolbar(mut registry: ResMut<ToolbarRegistry>) {
    registry.tools.push(ToolbarTool {
        id: ToolId::Destroy,
        label: "Destroy".to_string(),
        order: 10,
        key: Some(KeyCode::Digit2),
    });
}

fn draw_destruction_ui(toolbar: Res<ToolbarState>, mut action_text: ResMut<ToolbarActionText>) {
    if toolbar.active_tool != Some(ToolId::Destroy) {
        return;
    }

    action_text.0 = "Mode: Destroy\nLMB: Remove hovered object".to_string();
}

fn draw_hover_highlight(
    mut gizmos: Gizmos,
    hovered: Res<HoveredObject>,
    types: Option<Res<ObjectTypes>>,
    toolbar: Res<ToolbarState>,
    terrain: Res<TerrainWorld>,
    q_objects: Query<(&Transform, &ObjectKind)>,
) {
    let Some(types) = types else {
        return;
    };

    if toolbar.active_tool != Some(ToolId::Destroy) {
        return;
    }

    let Some(entity) = hovered.0 else {
        return;
    };

    let Ok((transform, kind)) = q_objects.get(entity) else {
        return;
    };

    let Some(spec) = types.registry.get(kind.0) else {
        return;
    };

    let base_h = terrain.sample_height_at(transform.translation.x, transform.translation.z);

    gizmos.circle(
        Isometry3d::new(
            Vec3::new(
                transform.translation.x,
                base_h + 0.1,
                transform.translation.z,
            ),
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        spec.hover_radius,
        Color::WHITE,
    );
}

fn handle_destruction_click(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    toolbar: Res<ToolbarState>,
    hovered: Res<HoveredObject>,
    ui_capture: Res<UiInputCapture>,
    children: Query<&Children>,
) {
    if ui_capture.pointer {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if toolbar.active_tool == Some(ToolId::Destroy) {
        if let Some(entity) = hovered.0 {
            highlight::despawn_recursive(&mut commands, &children, entity);
        }
    }
}
