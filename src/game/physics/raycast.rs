use bevy::prelude::*;

pub fn raycast_to_heightfield(terrain: &terrain::world::TerrainWorld, ray: Ray3d) -> Option<Vec3> {
    // Only handle rays pointing downwards.
    if ray.direction.y >= -1e-4 {
        return None;
    }

    // We step along the ray until we go below the heightfield, then refine with binary search.
    // This avoids needing physics/collision meshes.
    let max_depth_y = -200.0;
    let t_max = ((ray.origin.y - max_depth_y) / (-ray.direction.y)).clamp(0.0, 10_000.0);
    if t_max <= 0.0 {
        return None;
    }

    let step_y = (terrain.config.tile_size * 0.5).clamp(0.25, 2.0);
    let step_t = (step_y / (-ray.direction.y)).clamp(0.01, 5.0);

    let mut prev_t = 0.0;
    let mut prev_p = ray.origin;
    let mut prev_h = terrain.sample_height_at(prev_p.x, prev_p.z);

    let mut t = step_t;
    while t <= t_max {
        let p = ray.origin + *ray.direction * t;
        let h = terrain.sample_height_at(p.x, p.z);

        if p.y <= h {
            // Bracketed: prev is above, current is below.
            let mut lo = prev_t;
            let mut hi = t;

            for _ in 0..12 {
                let mid = 0.5 * (lo + hi);
                let mp = ray.origin + ray.direction * mid;
                let mh = terrain.sample_height_at(mp.x, mp.z);
                if mp.y <= mh {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }

            let hit_t = hi;
            let hit_p = ray.origin + *ray.direction * hit_t;
            let hit_h = terrain.sample_height_at(hit_p.x, hit_p.z);
            return Some(Vec3::new(hit_p.x, hit_h, hit_p.z));
        }

        prev_t = t;
        prev_p = p;
        prev_h = h;
        t += step_t;
    }

    // If we started below the terrain (rare), treat it as a hit at origin projection.
    if prev_p.y <= prev_h {
        return Some(Vec3::new(prev_p.x, prev_h, prev_p.z));
    }

    None
}
