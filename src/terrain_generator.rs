pub trait TerrainGenerator {
    fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Vec<f32>;
}

pub struct FlatTerrainGenerator {
    chunk_size: usize,
}

impl TerrainGenerator for FlatTerrainGenerator {
    fn generate_chunk(&self, _chunk_x: i32, _chunk_z: i32) -> Vec<f32> {
        let vertex_count = (self.chunk_size + 1) * (self.chunk_size + 1);
        vec![0.0; vertex_count]
    }
}

impl FlatTerrainGenerator {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
}