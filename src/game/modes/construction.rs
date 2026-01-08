use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::game::camera::UiInputCaptureRes;
use crate::game::input::CursorHitRes;
use crate::game::ui::toolbar::{ToolbarActionText, ToolbarRegistry, ToolbarState, ToolbarTool};
use crate::game::utils::highlight;
use crate::game::world::objects::system::{FreeformObjectWorldRes, ObjectTypesRes};
use crate::game::world::objects::types::ObjectTypeId;
use crate::game::world::terrain::types::TerrainWorldRes;

#[derive(Resource, Default)]
pub struct ConstructionStateRes {
    /// Selected object for construction
    pub selected: Option<ObjectTypeId>,
}

#[derive(Resource, Default)]
pub struct PlacementRotationRes {
    pub yaw: f32,
}

#[derive(Resource)]
pub struct HologramMaterialsRes {
    pub valid: Handle<StandardMaterial>,
    pub blocked: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct HologramPreviewRes {
    pub entity: Option<Entity>,
}

pub struct ConstructionModePlugin;

impl Plugin for ConstructionModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementRotationRes>()
            .init_resource::<HologramPreviewRes>()
            .init_resource::<ConstructionStateRes>()
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
        id: "construct".to_string(),
        label: "Construct".to_string(),
        order: 0,
        key: Some(KeyCode::Digit1),
    });
}

fn reset_on_tool_change(
    toolbar: Res<ToolbarState>,
    mut construction: ResMut<ConstructionStateRes>,
) {
    if toolbar.is_changed() && toolbar.active_tool.as_deref() != Some("construct") {
        construction.selected = None;
    }
}

fn setup_construction_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(HologramMaterialsRes {
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
    mut rot: ResMut<PlacementRotationRes>,
    ui_capture: Res<UiInputCaptureRes>,
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
    terrain: Res<TerrainWorldRes>,
    asset_server: Res<AssetServer>,
    types: Res<ObjectTypesRes>,
    objects: Res<FreeformObjectWorldRes>,
    toolbar: Res<ToolbarState>,
    construction: Res<ConstructionStateRes>,
    hit: Res<CursorHitRes>,
    placement_rot: Res<PlacementRotationRes>,
    hologram_materials: Res<HologramMaterialsRes>,
    mut preview: ResMut<HologramPreviewRes>,
    children: Query<&Children>,
    mut q_materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let show = toolbar.active_tool.as_deref() == Some("construct")
        && hit.world.is_some()
        && construction.selected.is_some();
    if !show {
        if let Some(e) = preview.entity.take() {
            highlight::despawn_recursive(&mut commands, &children, e);
        }
        return;
    }

    let Some(object_type) = construction.selected else {
        return;
    };

    let Some(spec) = types.registry.get(object_type) else {
        return;
    };
    if spec.gltf.trim().is_empty() {
        return;
    }
    let Some(hit_world) = hit.world else {
        return;
    };

    let base_h = terrain.0.sample_height_at(hit_world.x, hit_world.z);
    let rot = Quat::from_rotation_y(placement_rot.yaw);
    let rotated_offset = rot
        * Vec3::new(
            spec.render_offset.x,
            spec.render_offset.y,
            spec.render_offset.z,
        );

    let pos_world = Vec3::new(hit_world.x, base_h, hit_world.z) + rotated_offset;
    let transform = Transform::from_translation(pos_world)
        .with_rotation(rot)
        .with_scale(spec.render_scale);

    let can_place = objects
        .0
        .can_place_non_overlapping(&types.registry, object_type, hit_world);

    let chosen_material = if can_place {
        &hologram_materials.valid
    } else {
        &hologram_materials.blocked
    };

    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(spec.gltf.clone()));

    let preview_entity =
        highlight::update_hologram(&mut commands, preview.entity, scene_handle, transform);
    preview.entity = Some(preview_entity);

    highlight::apply_hologram_material_recursive(
        &children,
        &mut q_materials,
        preview_entity,
        chosen_material,
        0,
    );
}

fn handle_construction_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    hit: Res<CursorHitRes>,
    toolbar: Res<ToolbarState>,
    construction: Res<ConstructionStateRes>,
    placement_rot: Res<PlacementRotationRes>,
    types: Res<ObjectTypesRes>,
    mut objects: ResMut<FreeformObjectWorldRes>,
    ui_capture: Res<UiInputCaptureRes>,
) {
    if ui_capture.pointer {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if toolbar.active_tool.as_deref() != Some("construct") {
        return;
    }

    if let Some(object) = construction.selected {
        let Some(world) = hit.world else {
            return;
        };
        if objects
            .0
            .can_place_non_overlapping(&types.registry, object, world)
        {
            let _ = objects.0.place(object, world, placement_rot.yaw);
        }
    }
}

fn draw_construction_ui(
    mut contexts: EguiContexts,
    toolbar: Res<ToolbarState>,
    mut construction: ResMut<ConstructionStateRes>,
    types: Res<ObjectTypesRes>,
    mut action_text: ResMut<ToolbarActionText>,
) {
    if toolbar.active_tool.as_deref() != Some("construct") {
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
