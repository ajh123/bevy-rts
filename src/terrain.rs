use glam::{IVec2, Vec2, Vec3};
use parrot::Perlin;
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Debug)]
pub struct TerrainConfig {
    pub seed: u64,
    pub chunk_size: i32,
    pub tile_size: f32,
    pub view_distance_chunks: i32,
    pub chunk_spawn_budget_per_frame: usize,
    pub noise_base_frequency: f64,
    pub noise_octaves: u32,
    pub noise_persistence: f64,
    pub height_scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainAction {
    SpawnChunk(IVec2),
    DespawnChunk(IVec2),
}

#[derive(Clone, Debug)]
pub struct ChunkMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[derive(Default)]
struct ChunkStreamingState {
    last_viewer_chunk: Option<IVec2>,
    desired: HashSet<IVec2>,
    pending_spawn: VecDeque<IVec2>,
    pending_despawn: VecDeque<IVec2>,
}

pub struct TerrainWorld {
    pub config: TerrainConfig,
    perlin: Perlin,
    loaded: HashSet<IVec2>,
    streaming: ChunkStreamingState,
    viewer_world_xz: Vec2,
}

impl TerrainWorld {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            perlin: Perlin::new(config.seed),
            config,
            loaded: HashSet::new(),
            streaming: ChunkStreamingState::default(),
            viewer_world_xz: Vec2::ZERO,
        }
    }

    pub fn set_viewer_world_xz(&mut self, world_xz: Vec2) {
        self.viewer_world_xz = world_xz;
    }

    pub fn tick(&mut self) -> Vec<TerrainAction> {
        let chunk_world_size = self.config.chunk_size as f32 * self.config.tile_size;
        let viewer_chunk = IVec2::new(
            (self.viewer_world_xz.x / chunk_world_size).floor() as i32,
            (self.viewer_world_xz.y / chunk_world_size).floor() as i32,
        );

        // Recompute streaming targets only when entering a new chunk.
        if self.streaming.last_viewer_chunk != Some(viewer_chunk) {
            self.streaming.last_viewer_chunk = Some(viewer_chunk);

            self.streaming.desired.clear();
            for dz in -self.config.view_distance_chunks..=self.config.view_distance_chunks {
                for dx in -self.config.view_distance_chunks..=self.config.view_distance_chunks {
                    self.streaming.desired.insert(viewer_chunk + IVec2::new(dx, dz));
                }
            }

            self.streaming.pending_spawn.clear();
            for coord in self.streaming.desired.iter().copied() {
                if !self.loaded.contains(&coord) {
                    self.streaming.pending_spawn.push_back(coord);
                }
            }

            self.streaming.pending_despawn.clear();
            for coord in self.loaded.iter().copied() {
                if !self.streaming.desired.contains(&coord) {
                    self.streaming.pending_despawn.push_back(coord);
                }
            }
        }

        let mut actions = Vec::new();

        // Incremental despawn/spawn to avoid massive spikes at large view distances.
        let mut budget = self.config.chunk_spawn_budget_per_frame;
        while budget > 0 {
            let Some(coord) = self.streaming.pending_despawn.pop_front() else {
                break;
            };
            if self.loaded.remove(&coord) {
                actions.push(TerrainAction::DespawnChunk(coord));
            }
            budget -= 1;
        }

        let mut budget = self.config.chunk_spawn_budget_per_frame;
        while budget > 0 {
            let Some(coord) = self.streaming.pending_spawn.pop_front() else {
                break;
            };
            if self.loaded.contains(&coord) {
                budget -= 1;
                continue;
            }
            self.loaded.insert(coord);
            actions.push(TerrainAction::SpawnChunk(coord));
            budget -= 1;
        }

        actions
    }

    pub fn chunk_origin_world(&self, coord: IVec2) -> Vec3 {
        let chunk_world_size = self.config.chunk_size as f32 * self.config.tile_size;
        Vec3::new(
            coord.x as f32 * chunk_world_size,
            0.0,
            coord.y as f32 * chunk_world_size,
        )
    }

    pub fn build_chunk_mesh_data(&self, coord: IVec2, atlas_tile_count: f32) -> ChunkMeshData {
        let chunk_world_size = self.config.chunk_size as f32 * self.config.tile_size;
        let chunk_origin_x = coord.x as f32 * chunk_world_size;
        let chunk_origin_z = coord.y as f32 * chunk_world_size;

        let n = self.config.chunk_size.max(1) as usize;
        let stride = n + 1;
        let tile_size = self.config.tile_size;

        // Pre-sample heights once per grid vertex (huge perf win vs per-tile sampling).
        let mut heights: Vec<f32> = vec![0.0; stride * stride];
        for gz in 0..=n {
            for gx in 0..=n {
                let wx = chunk_origin_x + gx as f32 * tile_size;
                let wz = chunk_origin_z + gz as f32 * tile_size;
                heights[gz * stride + gx] = sample_height(&self.config, &self.perlin, wx, wz);
            }
        }

        // Derive smooth normals from the height grid (no extra noise samples).
        let mut normals_grid: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; stride * stride];
        for gz in 0..=n {
            for gx in 0..=n {
                let gx_l = gx.saturating_sub(1);
                let gx_r = (gx + 1).min(n);
                let gz_d = gz.saturating_sub(1);
                let gz_u = (gz + 1).min(n);

                let h_l = heights[gz * stride + gx_l];
                let h_r = heights[gz * stride + gx_r];
                let h_d = heights[gz_d * stride + gx];
                let h_u = heights[gz_u * stride + gx];

                let dx = ((gx_r as i32 - gx_l as i32).max(1) as f32) * tile_size;
                let dz = ((gz_u as i32 - gz_d as i32).max(1) as f32) * tile_size;

                let dhdx = (h_r - h_l) / dx;
                let dhdz = (h_u - h_d) / dz;

                let normal = Vec3::new(-dhdx, 1.0, -dhdz).normalize_or_zero();
                normals_grid[gz * stride + gx] = [normal.x, normal.y, normal.z];
            }
        }

        let tile_count = (n * n) as usize;
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(tile_count * 4);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(tile_count * 4);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(tile_count * 4);
        let mut indices: Vec<u32> = Vec::with_capacity(tile_count * 6);

        for z in 0..n {
            for x in 0..n {
                let x0 = x as f32 * tile_size;
                let z0 = z as f32 * tile_size;
                let x1 = x0 + tile_size;
                let z1 = z0 + tile_size;

                let h00 = heights[z * stride + x];
                let h10 = heights[z * stride + (x + 1)];
                let h01 = heights[(z + 1) * stride + x];
                let h11 = heights[(z + 1) * stride + (x + 1)];

                let n00 = normals_grid[z * stride + x];
                let n10 = normals_grid[z * stride + (x + 1)];
                let n01 = normals_grid[(z + 1) * stride + x];
                let n11 = normals_grid[(z + 1) * stride + (x + 1)];

                let avg_h = (h00 + h10 + h01 + h11) * 0.25;
                let tile_index = pick_tile_index(avg_h);
                let uv_u = (tile_index as f32 + 0.5) / atlas_tile_count;
                let uv = [uv_u, 0.5];

                let v0 = Vec3::new(x0, h00, z0);
                let v1 = Vec3::new(x1, h10, z0);
                let v2 = Vec3::new(x0, h01, z1);
                let v3 = Vec3::new(x1, h11, z1);

                let base = positions.len() as u32;
                positions.extend_from_slice(&[
                    [v0.x, v0.y, v0.z],
                    [v1.x, v1.y, v1.z],
                    [v2.x, v2.y, v2.z],
                    [v3.x, v3.y, v3.z],
                ]);
                normals.extend_from_slice(&[n00, n10, n01, n11]);
                uvs.extend_from_slice(&[uv, uv, uv, uv]);

                // Winding chosen so the "top" faces upward (CCW when viewed from above).
                indices.extend_from_slice(&[
                    base,
                    base + 2,
                    base + 1,
                    base + 1,
                    base + 2,
                    base + 3,
                ]);
            }
        }

        ChunkMeshData {
            positions,
            normals,
            uvs,
            indices,
        }
    }
}

fn sample_height(config: &TerrainConfig, perlin: &Perlin, world_x: f32, world_z: f32) -> f32 {
    let mut amplitude = 1.0f64;
    let mut frequency = config.noise_base_frequency;
    let mut sum = 0.0f64;
    let mut norm = 0.0f64;

    for _ in 0..config.noise_octaves {
        let n = perlin.noise2d(world_x as f64 * frequency, world_z as f64 * frequency);
        sum += n * amplitude;
        norm += amplitude;
        amplitude *= config.noise_persistence;
        frequency *= 2.0;
    }

    let value = if norm > 0.0 { sum / norm } else { 0.0 };
    (value as f32) * config.height_scale
}

fn pick_tile_index(height: f32) -> u32 {
    // 0..=4 maps to the atlas order: [water, sand, grass, rock, snow]
    if height < -3.0 {
        0
    } else if height < -1.0 {
        1
    } else if height < 3.0 {
        2
    } else if height < 6.0 {
        3
    } else {
        4
    }
}
