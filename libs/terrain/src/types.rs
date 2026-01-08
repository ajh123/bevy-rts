use bevy::prelude::*;
use glam::IVec2;
use glam::Vec2;
use serde::Deserialize;
use std::collections::HashMap;

// --- Config ---

#[derive(Resource, Clone, Debug)]
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

// --- Tiles ---

#[derive(Clone, Debug, Deserialize)]
pub struct TileTypesFile {
    pub tiles: Vec<TileType>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TileType {
    pub name: String,
    pub color_srgb: (f32, f32, f32),
    /// Select this tile if height < height_lt.
    pub height_lt: f32,
}

#[derive(Resource, Clone, Debug)]
pub struct TileTypes {
    pub tiles: Vec<TileType>,
}

impl TileTypes {
    pub fn tile_count_f32(&self) -> f32 {
        self.tiles.len() as f32
    }

    pub fn pick_tile_index(&self, height: f32) -> u32 {
        // Validation guarantees there's at least 1 tile.
        for (i, t) in self.tiles.iter().enumerate() {
            if height < t.height_lt {
                return i as u32;
            }
        }
        (self.tiles.len().saturating_sub(1)) as u32
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.tiles.is_empty() {
            return Err("tile types file must define at least one tile".to_string());
        }

        let mut last = std::f32::NEG_INFINITY;
        for t in &self.tiles {
            if !t.height_lt.is_finite() {
                return Err(format!("tile '{}' has non-finite height_lt", t.name));
            }
            if t.height_lt <= last {
                return Err(format!(
                    "tile '{}' has height_lt={} but previous tile had height_lt={} (must be strictly increasing)",
                    t.name, t.height_lt, last
                ));
            }
            last = t.height_lt;
        }

        Ok(())
    }
}

// --- Resources ---

#[derive(Resource)]
pub struct TerrainAtlas {
    pub material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct LoadedChunkEntities {
    pub entities: HashMap<IVec2, Entity>,
}

/// Set by the root game crate to indicate where the viewer is (XZ plane).
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct TerrainViewerWorldXz(pub Vec2);
