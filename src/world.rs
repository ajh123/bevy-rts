use std::collections::HashMap;
use crate::terrain_generator::TerrainGenerator;

pub const CHUNK_SIZE: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkKey {
    pub x: i32,
    pub z: i32,
}

impl ChunkKey {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn from_world_position(pos_x: f32, pos_z: f32) -> Self {
        let x = (pos_x / CHUNK_SIZE as f32).floor() as i32;
        let z = (pos_z / CHUNK_SIZE as f32).floor() as i32;
        Self::new(x, z)
    }
}

pub struct Chunk {
    pub heights: Vec<f32>,
    pub dirty: bool,
}

impl Chunk {
    pub fn new(terrain_generator: &dyn TerrainGenerator, key: ChunkKey) -> Self {
        Self {
            heights: terrain_generator.generate_chunk(key.x, key.z),
            dirty: true,
        }
    }

    pub fn get_grid_position(vertex_index: usize) -> (usize, usize) {
        (vertex_index % (CHUNK_SIZE + 1), vertex_index / (CHUNK_SIZE + 1))
    }

    pub fn vertex_count() -> usize {
        (CHUNK_SIZE + 1) * (CHUNK_SIZE + 1)
    }

    pub fn index_count() -> usize {
        CHUNK_SIZE * CHUNK_SIZE * 6
    }

    pub fn generate_indices() -> Vec<u16> {
        let mut indices = Vec::with_capacity(Self::index_count());
        for iz in 0..CHUNK_SIZE {
            for ix in 0..CHUNK_SIZE {
                let top_left = iz * (CHUNK_SIZE + 1) + ix;
                let top_right = top_left + 1;
                let bottom_left = (iz + 1) * (CHUNK_SIZE + 1) + ix;
                let bottom_right = bottom_left + 1;

                indices.extend_from_slice(&[top_left as u16, bottom_left as u16, top_right as u16]);
                indices.extend_from_slice(&[top_right as u16, bottom_left as u16, bottom_right as u16]);
            }
        }
        indices
    }
}

pub struct World {
    pub chunks: HashMap<ChunkKey, Chunk>,
    terrain_generator: Box<dyn TerrainGenerator>,
}

impl World {
    pub fn new(terrain_generator: Box<dyn TerrainGenerator>) -> Self {
        Self {
            chunks: HashMap::new(),
            terrain_generator,
        }
    }

    pub fn update(&mut self, camera_pos_x: f32, camera_pos_z: f32, render_distance: i32) {
        let center_chunk = ChunkKey::from_world_position(camera_pos_x, camera_pos_z);

        for dz in -render_distance..=render_distance {
            for dx in -render_distance..=render_distance {
                let dist = (dx.abs() + dz.abs()) as f32;
                if dist <= render_distance as f32 {
                    let key = ChunkKey::new(center_chunk.x + dx, center_chunk.z + dz);

                    if !self.chunks.contains_key(&key) {
                        let chunk = Chunk::new(self.terrain_generator.as_ref(), key);
                        self.chunks.insert(key, chunk);
                    }
                }
            }
        }

        self.chunks.retain(|key, _| {
            let dist_x = (key.x - center_chunk.x).abs();
            let dist_z = (key.z - center_chunk.z).abs();
            dist_x <= render_distance && dist_z <= render_distance
        });
    }

    pub fn chunks_iter(&self) -> impl Iterator<Item = (&ChunkKey, &Chunk)> {
        self.chunks.iter()
    }

    pub fn chunks_iter_mut(&mut self) -> impl Iterator<Item = (&ChunkKey, &mut Chunk)> {
        self.chunks.iter_mut()
    }
}
