use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use objects::ObjectTypeId;
use objects::highlight;
use objects::spatial::SpatialHashGrid;
use objects::system::{CursorHit, ObjectKind, ObjectTypes};
use terrain::TerrainWorld;
use ui::{ToolId, ToolbarActionText, ToolbarRegistry, ToolbarState, ToolbarTool, UiInputCapture};

#[derive(Resource, Default)]
pub struct ConstructionState {
    /// Selected object for construction
    pub selected: Option<ObjectTypeId>,
}

#[derive(Resource, Default)]
pub struct PlacementRotation {
    pub yaw: f32,
}

#[derive(Resource)]
pub struct HologramMaterials {
    pub valid: Handle<StandardMaterial>,
    pub blocked: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct HologramPreview {
    pub entity: Option<Entity>,
    pub scene_child: Option<Entity>,
    pub object_type: Option<ObjectTypeId>,
}

pub struct ConstructionModePlugin;

impl Plugin for ConstructionModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementRotation>()
            .init_resource::<HologramPreview>()
            .init_resource::<ConstructionState>()
            .add_systems(
                Startup,
                (setup_construction_materials, setup_construction_toolbar),
            )
            .add_systems(
                Update,
                (
                    update_placement_rotation,
                    update_hologram_preview,
                    handle_construction_click,
                    reset_on_tool_change,
                ),
            )
            .add_systems(EguiPrimaryContextPass, draw_construction_ui);
    }
}

fn setup_construction_toolbar(mut registry: ResMut<ToolbarRegistry>) {
    registry.tools.push(ToolbarTool {
        id: ToolId::Construct,
        label: "Construct".to_string(),
        order: 0,
        key: Some(KeyCode::Digit1),
    });
}

fn reset_on_tool_change(
    toolbar: Res<ToolbarState>,
    mut construction: ResMut<ConstructionState>,
    mut preview: ResMut<HologramPreview>,
) {
    if toolbar.is_changed() && toolbar.active_tool != Some(ToolId::Construct) {
        construction.selected = None;
        preview.object_type = None;
    }
}

fn setup_construction_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(HologramMaterials {
        valid: materials.add(StandardMaterial {
            base_color: Color::hsla(180.0, 0.8, 0.5, 0.5),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        blocked: materials.add(StandardMaterial {
            base_color: Color::hsla(0.0, 0.8, 0.5, 0.5),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    });
}

fn update_placement_rotation(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut rot: ResMut<PlacementRotation>,
    ui_capture: Res<UiInputCapture>,
) {
    if ui_capture.keyboard {
        return;
    }

    let mut delta: f32 = 0.0;
    if keys.pressed(KeyCode::KeyR) {
        delta += 1.0;
    }
    if keys.pressed(KeyCode::KeyF) {
        delta -= 1.0;
    }

    if delta.abs() > 0.0 {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            3.5
        } else {
            1.75
        };
        rot.yaw = (rot.yaw + delta * speed * time.delta_secs()).rem_euclid(std::f32::consts::TAU);
    }
}

fn update_hologram_preview(
    mut commands: Commands,
    terrain: Res<TerrainWorld>,
    asset_server: Res<AssetServer>,
    types: Option<Res<ObjectTypes>>,
    q_objects: Query<(&Transform, &ObjectKind)>,
    toolbar: Res<ToolbarState>,
    construction: Res<ConstructionState>,
    hit: Res<CursorHit>,
    placement_rot: Res<PlacementRotation>,
    hologram_materials: Res<HologramMaterials>,
    mut preview: ResMut<HologramPreview>,
    children: Query<&Children>,
    mut q_materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    grid: Res<SpatialHashGrid>,
) {
    let Some(types) = types else {
        return;
    };

    let show = toolbar.active_tool == Some(ToolId::Construct)
        && hit.world.is_some()
        && construction.selected.is_some();
    if !show {
        if let Some(e) = preview.entity.take() {
            highlight::despawn_recursive(&mut commands, &children, e);
        }
        preview.scene_child = None;
        preview.object_type = None;
        return;
    }

    let Some(object_type) = construction.selected else {
        return;
    };

    // If the selected object changes, respawn the preview so we don't keep the old scene.
    if preview.object_type != Some(object_type) {
        if let Some(e) = preview.entity.take() {
            highlight::despawn_recursive(&mut commands, &children, e);
        }
        preview.scene_child = None;
        preview.object_type = Some(object_type);
    }

    let Some(spec) = types.registry.get(object_type) else {
        return;
    };
    if spec.gltf.trim().is_empty() {
        return;
    }
    let Some(hit_world) = hit.world else {
        return;
    };

    let base_h = terrain.sample_height_at(hit_world.x, hit_world.z);
    let pos_world = Vec3::new(hit_world.x, base_h, hit_world.z);
    let rot = Quat::from_rotation_y(placement_rot.yaw);
    let transform = Transform::from_translation(pos_world)
        .with_rotation(rot)
        .with_scale(spec.render_scale);

    let can_place = objects::system::can_place_non_overlapping_spatial(
        &types.registry,
        object_type,
        pos_world,
        &grid,
        &q_objects,
    );

    let chosen_material = if can_place {
        &hologram_materials.valid
    } else {
        &hologram_materials.blocked
    };

    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));

    let scene_offset_local = spec.scene_offset_local;

    let (preview_entity, scene_child) = highlight::update_hologram(
        &mut commands,
        preview.entity,
        preview.scene_child,
        scene_handle,
        transform,
        scene_offset_local,
    );
    preview.entity = Some(preview_entity);
    preview.scene_child = Some(scene_child);

    highlight::apply_hologram_material_recursive(
        &children,
        &mut q_materials,
        preview_entity,
        chosen_material,
        0,
    );
}

fn handle_construction_click(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    hit: Res<CursorHit>,
    toolbar: Res<ToolbarState>,
    construction: Res<ConstructionState>,
    placement_rot: Res<PlacementRotation>,
    types: Option<Res<ObjectTypes>>,
    q_objects: Query<(&Transform, &ObjectKind)>,
    terrain: Res<TerrainWorld>,
    asset_server: Res<AssetServer>,
    ui_capture: Res<UiInputCapture>,
    grid: Res<SpatialHashGrid>,
) {
    let Some(types) = types else {
        return;
    };

    if ui_capture.pointer {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if toolbar.active_tool != Some(ToolId::Construct) {
        return;
    }

    if let Some(object) = construction.selected {
        let Some(world) = hit.world else {
            return;
        };
        let base_h = terrain.sample_height_at(world.x, world.z);
        let position = Vec3::new(world.x, base_h, world.z);

        let can_place = objects::system::can_place_non_overlapping_spatial(
            &types.registry,
            object,
            position,
            &grid,
            &q_objects,
        );
        if can_place {
            let _ = objects::system::spawn_object(
                &mut commands,
                &types.registry,
                &asset_server,
                object,
                position,
                placement_rot.yaw,
            );
        }
    }
}

fn draw_construction_ui(
    mut contexts: EguiContexts,
    toolbar: Res<ToolbarState>,
    mut construction: ResMut<ConstructionState>,
    types: Option<Res<ObjectTypes>>,
    mut action_text: ResMut<ToolbarActionText>,
) {
    let Some(types) = types else {
        return;
    };

    if toolbar.active_tool != Some(ToolId::Construct) {
        return;
    }

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };
    let toolbar_width = 360.0;
    let toolbar_height = 40.0;
    let secondary_height = 52.0;
    let margin = 10.0;

    let viewport = ctx.viewport_rect();

    egui::Area::new("bottom_toolbar_secondary".into())
        .fixed_pos(egui::pos2(
            (viewport.width() - toolbar_width) / 2.0,
            viewport.height() - toolbar_height - secondary_height - margin * 2.0,
        ))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(45, 45, 45))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(95, 95, 95)))
                .corner_radius(6)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(toolbar_width, secondary_height));
                    ui.set_max_width(toolbar_width);

                    egui::ScrollArea::horizontal()
                        .auto_shrink([false, true])
                        .max_width(toolbar_width)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for id in types.available.iter().copied() {
                                    let name = types
                                        .registry
                                        .get(id)
                                        .map(|s| s.name.as_str())
                                        .unwrap_or("Object");

                                    let is_selected = construction.selected == Some(id);
                                    if ui
                                        .add(egui::Button::new(name).selected(is_selected))
                                        .clicked()
                                    {
                                        if is_selected {
                                            construction.selected = None;
                                        } else {
                                            construction.selected = Some(id);
                                        }
                                    }
                                }
                            });
                        });
                });
        });

    let text = match construction.selected {
        None => {
            let mut s = String::new();
            s.push_str("Mode: Construct\n");
            s.push_str("Select a model above");
            s
        }
        Some(object) => {
            let name = types
                .registry
                .get(object)
                .map(|s| s.name.as_str())
                .unwrap_or("Object");
            let mut s = String::new();
            s.push_str(&format!("Mode: Construct ({name})\n"));
            s.push_str("LMB: Place\n");
            s.push_str("R / F: Rotate (hold Shift for faster)");
            s
        }
    };
    action_text.0 = text;
}
