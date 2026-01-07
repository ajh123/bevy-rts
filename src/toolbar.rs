use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};


use crate::camera::UiInputCaptureRes;
use crate::object_system::{ObjectTypeId, ObjectTypesRes};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ToolbarMode {
    #[default]
    None,
    Construct { object: ObjectTypeId },
    Destroy,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub(crate) struct ToolbarState {
    pub(crate) mode: ToolbarMode,
    /// Last-selected construction object, used when switching back into construction mode.
    pub(crate) last_construct_object: Option<ObjectTypeId>,
}

impl ToolbarState {
    pub(crate) fn set_construct(&mut self, object: ObjectTypeId) {
        self.mode = ToolbarMode::Construct { object };
        self.last_construct_object = Some(object);
    }

    pub(crate) fn set_destroy(&mut self) {
        self.mode = ToolbarMode::Destroy;
    }

    pub(crate) fn set_none(&mut self) {
        self.mode = ToolbarMode::None;
    }
}

pub(crate) fn update_toolbar_state_from_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut toolbar: ResMut<ToolbarState>,
    types: Res<ObjectTypesRes>,
    ui_capture: Res<UiInputCaptureRes>,
) {
    if ui_capture.keyboard {
        return;
    }

    if keys.just_pressed(KeyCode::Digit1) {
        let object = toolbar.last_construct_object.unwrap_or(types.test_building);
        toolbar.set_construct(object);
    }

    if keys.just_pressed(KeyCode::Digit2) {
        toolbar.set_destroy();
    }
}

pub(crate) fn init_toolbar_state(mut toolbar: ResMut<ToolbarState>, types: Res<ObjectTypesRes>) {
    if toolbar.last_construct_object.is_none() {
        toolbar.last_construct_object = Some(types.test_building);
    }

    if matches!(toolbar.mode, ToolbarMode::None) {
        toolbar.set_construct(types.test_building);
    }
}

pub(crate) fn bottom_toolbar_system(
    mut contexts: EguiContexts,
    mut toolbar: ResMut<ToolbarState>,
    types: Res<ObjectTypesRes>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let toolbar_width = 360.0;
    let toolbar_height = 40.0;
    let margin = 10.0;

    let viewport = ctx.viewport_rect();

    // Bottom-centered toolbar
    egui::Area::new("bottom_toolbar".into())
        .fixed_pos(egui::pos2(
            (viewport.width() - toolbar_width) / 2.0,
            viewport.height() - toolbar_height - margin,
        ))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(50, 50, 50))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)))
                .corner_radius(6)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(toolbar_width, toolbar_height));

                    ui.horizontal_centered(|ui| {
                        let construct_object = toolbar
                            .last_construct_object
                            .unwrap_or(types.test_building);

                        let construct_name = types
                            .registry
                            .get(construct_object)
                            .map(|s| s.name.as_str())
                            .unwrap_or("Object");

                        let is_construct = matches!(toolbar.mode, ToolbarMode::Construct { .. });
                        let is_destroy = matches!(toolbar.mode, ToolbarMode::Destroy);

                        if ui
                            .add(egui::Button::new(format!(
                                "Construct: {construct_name} (1)"
                            ))
                            .selected(is_construct))
                            .clicked()
                        {
                            toolbar.set_construct(construct_object);
                        }

                        if ui
                            .add(egui::Button::new("Destroy (2)").selected(is_destroy))
                            .clicked()
                        {
                            toolbar.set_destroy();
                        }
                    });
                });
        });

    // Left bottom corner control information box (derived from toolbar state)
    let info_width = 340.0;
    let info_height = 110.0;

    egui::Area::new("control_info".into())
        .fixed_pos(egui::pos2(
            margin,
            viewport.height() - info_height - margin,
        ))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(35, 35, 35))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 90, 90)))
                .corner_radius(6)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(info_width, info_height));

                    match toolbar.mode {
                        ToolbarMode::Construct { object } => {
                            let name = types
                                .registry
                                .get(object)
                                .map(|s| s.name.as_str())
                                .unwrap_or("Object");
                            ui.label(format!("Mode: Construct ({name})"));
                            ui.label("LMB: Place");
                            ui.label("R / F: Rotate (hold Shift for faster)");
                            ui.label("2: Destroy");
                        }
                        ToolbarMode::Destroy => {
                            ui.label("Mode: Destroy");
                            ui.label("LMB: Remove hovered object");
                            ui.label("1: Construct");
                        }
                        ToolbarMode::None => {
                            ui.label("Mode: None");
                            ui.label("1: Construct");
                            ui.label("2: Destroy");
                        }
                    }
                });
        });

    Ok(())
}
