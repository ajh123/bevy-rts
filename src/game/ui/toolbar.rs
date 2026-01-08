use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::game::camera::UiInputCaptureRes;

#[derive(Resource, Default)]
pub struct ToolbarRegistry {
    pub tools: Vec<ToolbarTool>,
}

pub struct ToolbarTool {
    pub id: String,
    pub label: String,
    pub order: u32,
    pub key: Option<KeyCode>,
}

#[derive(Resource, Debug, Default)]
pub struct ToolbarState {
    pub active_tool: Option<String>,
}

#[derive(Resource, Default)]
pub struct ToolbarActionText(pub String);

fn format_key(key: KeyCode) -> String {
    let s = format!("{:?}", key);
    if let Some(d) = s.strip_prefix("Digit") {
        return d.to_string();
    }
    if let Some(k) = s.strip_prefix("Key") {
        return k.to_string();
    }
    s
}

pub(crate) fn update_toolbar_state_from_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut toolbar: ResMut<ToolbarState>,
    registry: Res<ToolbarRegistry>,
    ui_capture: Res<UiInputCaptureRes>,
) {
    if ui_capture.keyboard {
        return;
    }

    for tool in &registry.tools {
        if let Some(key) = tool.key {
            if keys.just_pressed(key) {
                 if toolbar.active_tool.as_ref() == Some(&tool.id) {
                    toolbar.active_tool = None;
                } else {
                    toolbar.active_tool = Some(tool.id.clone());
                }
            }
        }
    }
}

pub(crate) fn bottom_toolbar_system(
    mut contexts: EguiContexts,
    mut toolbar: ResMut<ToolbarState>,
    registry: Res<ToolbarRegistry>,
    action_text: Res<ToolbarActionText>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let toolbar_width = 360.0;
    let toolbar_height = 40.0;
    let margin = 10.0;

    let viewport = ctx.viewport_rect();

    // Info box
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
                    
                    if toolbar.active_tool.is_none() {
                        ui.label("Mode: None");
                        
                        let mut sorted_tools: Vec<&ToolbarTool> = registry.tools.iter().collect();
                        sorted_tools.sort_by_key(|t| t.order);

                        for tool in sorted_tools.into_iter() {
                            let key_help = tool.key.map(format_key).unwrap_or_default();
                            let prefix = if key_help.is_empty() { "".to_string() } else { format!("{}: ", key_help) };
                            ui.label(format!("{}{}", prefix, tool.label));
                        }
                    } else {
                        ui.label(&action_text.0);
                    }
                });
        });

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
                        let mut sorted_tools: Vec<_> = registry.tools.iter().collect();
                        sorted_tools.sort_by_key(|t| t.order);

                        for tool in sorted_tools.iter() {
                             let is_active = toolbar.active_tool.as_ref() == Some(&tool.id);
                             let key_hint = tool.key.map(|k| format!(" ({})", format_key(k))).unwrap_or_default();
                             let label = format!("{}{}", tool.label, key_hint);
                             if ui.add(egui::Button::new(label).selected(is_active)).clicked() {
                                 if is_active {
                                     toolbar.active_tool = None;
                                 } else {
                                     toolbar.active_tool = Some(tool.id.clone());
                                 }
                             }
                        }
                    });
                });
        });
}
