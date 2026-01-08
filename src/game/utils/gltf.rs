use bevy::prelude::*;
use glam::{Mat4, Vec3};
use serde_json::Value;
use crate::game::world::objects::types::GltfBounds;

/// Computes a standard rendering scale, offset, and collision radius from glTF AABB bounds.
/// 
/// This is used to normalize models that might be authored at weird scales or offsets.
pub fn compute_render_params(_tile_size: f32, bounds: Option<GltfBounds>, scale: Vec3) -> (Vec3, Vec3, f32) {
    // Use raw glTF units as-authored (no tile-size-based scaling).
    // We still compute a reasonable render offset and hover radius from bounds when available.
    if let Some(b) = bounds {
        let s = scale;

        // Center in XZ and put the model's bottom on the ground (y = 0).
        let center = b.center();
        let min_y = b.min.y;
        let offset = Vec3::new(-center.x * s.x, -min_y * s.y, -center.z * s.z);

        // Conservative radius in XZ for hover + collision.
        let size = b.size();
        let rx = 0.5 * size.x.abs() * s.x.abs();
        let rz = 0.5 * size.z.abs() * s.z.abs();
        let radius = (rx * rx + rz * rz).sqrt().max(0.1);

        (s, offset, radius)
    } else {
        // With unknown bounds we can't infer a size; keep scale 1 and pick a small sane radius.
        (scale, Vec3::ZERO, 1.0)
    }
}

/// Attempts to parse a .gltf file (not .glb) to determine its Axis Aligned Bounding Box.
/// This parses the JSON structure manually to find accessor min/max values.
pub fn try_compute_gltf_bounds_in_parent_space(asset_path: &str) -> Result<GltfBounds, String> {
    // Only supports JSON .gltf for now.
    if !asset_path.to_ascii_lowercase().ends_with(".gltf") {
        return Err("only .gltf is supported for bounds computation".to_string());
    }

    // Convert Bevy asset path (relative to assets/) into a filesystem path.
    let fs_path = std::path::Path::new("assets").join(asset_path);
    let text = std::fs::read_to_string(&fs_path)
        .map_err(|e| format!("failed to read gltf '{}': {e}", fs_path.display()))?;

    let doc: Value = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse gltf json '{}': {e}", fs_path.display()))?;

    let meshes = doc
        .get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "gltf missing 'meshes'".to_string())?;
    let accessors = doc
        .get("accessors")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "gltf missing 'accessors'".to_string())?;

    // Find accessor indices used as POSITION for primitives.
    let mut position_accessor_indices: Vec<usize> = Vec::new();
    for mesh in meshes {
        let primitives = match mesh.get("primitives").and_then(|v| v.as_array()) {
            Some(p) => p,
            None => continue,
        };
        for prim in primitives {
            let attrs = match prim.get("attributes").and_then(|v| v.as_object()) {
                Some(a) => a,
                None => continue,
            };
            let Some(pos_idx) = attrs.get("POSITION").and_then(|v| v.as_u64()) else {
                continue;
            };
            position_accessor_indices.push(pos_idx as usize);
        }
    }
    if position_accessor_indices.is_empty() {
        return Err("gltf has no POSITION accessors".to_string());
    }

    // Merge AABB across all POSITION accessors.
    let mut local_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut local_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

    for idx in position_accessor_indices {
        let Some(acc) = accessors.get(idx) else {
            continue;
        };
        let min = acc.get("min").and_then(|v| v.as_array());
        let max = acc.get("max").and_then(|v| v.as_array());
        let (Some(min), Some(max)) = (min, max) else {
            continue;
        };

        let read3 = |arr: &Vec<Value>| -> Option<Vec3> {
            Some(Vec3::new(
                arr.get(0)?.as_f64()? as f32,
                arr.get(1)?.as_f64()? as f32,
                arr.get(2)?.as_f64()? as f32,
            ))
        };

        let Some(min_v) = read3(min) else { continue; };
        let Some(max_v) = read3(max) else { continue; };

        local_min = local_min.min(min_v);
        local_max = local_max.max(max_v);
    }

    if !local_min.is_finite() || !local_max.is_finite() {
        return Err("failed to compute finite bounds from accessors".to_string());
    }

    // Apply default scene's root node matrix (if present) to get bounds in parent space.
    let root_transform = try_read_default_scene_root_matrix(&doc).unwrap_or(Mat4::IDENTITY);
    let (min_p, max_p) = transform_aabb(root_transform, local_min, local_max);

    Ok(GltfBounds { min: min_p, max: max_p })
}

fn try_read_default_scene_root_matrix(doc: &Value) -> Option<Mat4> {
    let scene_index = doc.get("scene").and_then(|v| v.as_u64())? as usize;
    let scenes = doc.get("scenes").and_then(|v| v.as_array())?;
    let scene = scenes.get(scene_index)?;
    let root_nodes = scene.get("nodes").and_then(|v| v.as_array())?;
    // Handle the common case: exactly one root node with a matrix.
    let root_idx = root_nodes.get(0)?.as_u64()? as usize;
    let nodes = doc.get("nodes").and_then(|v| v.as_array())?;
    let root = nodes.get(root_idx)?;

    if let Some(m) = root.get("matrix").and_then(|v| v.as_array()) {
        if m.len() == 16 {
            let mut f = [0.0f32; 16];
            for (i, v) in m.iter().enumerate() {
                f[i] = v.as_f64()? as f32;
            }
            // glTF matrices are column-major.
            return Some(Mat4::from_cols_array(&f));
        }
    }

    None
}

fn transform_aabb(m: Mat4, min: Vec3, max: Vec3) -> (Vec3, Vec3) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
    ];

    let mut out_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut out_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

    for c in corners {
        let p = m.transform_point3(c);
        out_min = out_min.min(p);
        out_max = out_max.max(p);
    }

    (out_min, out_max)
}
