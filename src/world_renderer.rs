use crate::world::{World, Chunk, ChunkKey, CHUNK_SIZE, GRID_SIZE};
use wgpu::util::DeviceExt;
use std::collections::HashMap;

struct ChunkBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

pub struct WorldRenderer {
    buffers: HashMap<ChunkKey, ChunkBuffers>,
    index_buffer: Option<wgpu::Buffer>,
    index_count: u32,
}

impl WorldRenderer {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            index_buffer: None,
            index_count: 0,
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, world: &mut World) {
        if self.index_buffer.is_none() {
            let indices = Chunk::generate_indices();
            self.index_count = indices.len() as u32;
            self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("World Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            }));
        }

        for (key, chunk) in world.chunks_iter_mut() {
            if chunk.dirty {
                let mut vertices = Vec::with_capacity(Chunk::vertex_count() * 6);
                let world_x = key.x as f32 * CHUNK_SIZE;
                let world_z = key.z as f32 * CHUNK_SIZE;
                let half_size = CHUNK_SIZE / 2.0;

                for (vertex_index, &height) in chunk.heights.iter().enumerate() {
                    let (ix, iz) = Chunk::get_grid_position(vertex_index);
                    let x = world_x - half_size + (ix as f32 / GRID_SIZE as f32) * CHUNK_SIZE;
                    let z = world_z - half_size + (iz as f32 / GRID_SIZE as f32) * CHUNK_SIZE;

                    let checkered = ((ix + iz) % 2 == 0) as u8 as f32;
                    let color = [0.4 + 0.2 * checkered, 0.6 + 0.2 * checkered, 0.4 + 0.2 * checkered];

                    vertices.extend_from_slice(&[x, height, z]);
                    vertices.extend_from_slice(&color);
                }

                if let Some(buffers) = self.buffers.get_mut(key) {
                    queue.write_buffer(&buffers.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
                } else {
                    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Chunk Vertex Buffer"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });

                    self.buffers.insert(*key, ChunkBuffers {
                        vertex_buffer,
                        index_buffer: self.index_buffer.as_ref().unwrap().clone(),
                        num_indices: self.index_count,
                    });
                }

                chunk.dirty = false;
            }
        }

        let active_keys: std::collections::HashSet<_> = world.chunks_iter().map(|(k, _)| *k).collect();
        self.buffers.retain(|key, _| active_keys.contains(key));
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for (_, buffers) in &self.buffers {
            render_pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
            render_pass.set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..buffers.num_indices, 0, 0..1);
        }
    }
}

impl Default for WorldRenderer {
    fn default() -> Self {
        Self::new()
    }
}
