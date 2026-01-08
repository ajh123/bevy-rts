use glam::Vec3;

/// Computes the simple collision/hover radius for a given object spec.
/// If bounds are available, uses the XZ diagonal. Otherwise uses hover_radius.
pub fn collision_radius(
    gltf_bounds: Option<&crate::game::world::objects::types::GltfBounds>,
    render_scale: Vec3,
    hover_radius: f32,
) -> f32 {
    if let Some(b) = gltf_bounds {
        let size = b.size();
        let sx = render_scale.x.abs();
        let sz = render_scale.z.abs();
        let rx = 0.5 * size.x.abs() * sx;
        let rz = 0.5 * size.z.abs() * sz;
        (rx * rx + rz * rz).sqrt().max(0.1)
    } else {
        hover_radius.max(0.1)
    }
}

/// Checks if two objects overlap based on their radii.
pub fn circles_overlap(p1: Vec3, r1: f32, p2: Vec3, r2: f32) -> bool {
    let dx = p1.x - p2.x;
    let dz = p1.z - p2.z;
    let dist_sq = dx * dx + dz * dz;
    let min_dist = (r1 + r2).max(0.01);
    dist_sq < (min_dist * min_dist)
}

/// Checks if a point is within a circle.
pub fn point_in_circle(p: Vec3, center: Vec3, radius: f32) -> bool {
    let dx = p.x - center.x;
    let dz = p.z - center.z;
    let dist_sq = dx * dx + dz * dz;
    dist_sq <= radius * radius
}
