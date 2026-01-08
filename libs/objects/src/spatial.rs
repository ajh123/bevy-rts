use bevy::prelude::*;
use glam::{IVec2, Vec2, Vec3};
use std::collections::{HashMap, HashSet};

use crate::system::ObjectKind;

#[derive(Resource, Debug)]
pub struct SpatialHashGrid {
    pub cell_size: f32,
    cells: HashMap<IVec2, Vec<Entity>>,
    entity_cell: HashMap<Entity, IVec2>,
}

impl Default for SpatialHashGrid {
    fn default() -> Self {
        Self {
            // Large enough to keep buckets small, small enough for local queries.
            cell_size: 8.0,
            cells: HashMap::new(),
            entity_cell: HashMap::new(),
        }
    }
}

impl SpatialHashGrid {
    pub fn cell_of_world(&self, world_xz: Vec2) -> IVec2 {
        let cs = self.cell_size.max(0.001);
        IVec2::new(
            (world_xz.x / cs).floor() as i32,
            (world_xz.y / cs).floor() as i32,
        )
    }

    pub fn insert_or_move(&mut self, entity: Entity, world_pos: Vec3) {
        let cell = self.cell_of_world(Vec2::new(world_pos.x, world_pos.z));

        if let Some(old) = self.entity_cell.insert(entity, cell) {
            if old == cell {
                return;
            }
            if let Some(list) = self.cells.get_mut(&old) {
                list.retain(|e| *e != entity);
            }
        }

        self.cells.entry(cell).or_default().push(entity);
    }

    pub fn remove(&mut self, entity: Entity) {
        let Some(cell) = self.entity_cell.remove(&entity) else {
            return;
        };
        if let Some(list) = self.cells.get_mut(&cell) {
            list.retain(|e| *e != entity);
        }
    }

    pub fn query_candidates(&self, world_xz: Vec2, radius: f32) -> Vec<Entity> {
        let cs = self.cell_size.max(0.001);
        let r = radius.max(0.0);
        let center = self.cell_of_world(world_xz);
        let reach = (r / cs).ceil() as i32;

        let mut out = Vec::new();
        let mut seen: HashSet<Entity> = HashSet::new();

        for dz in -reach..=reach {
            for dx in -reach..=reach {
                let c = center + IVec2::new(dx, dz);
                if let Some(list) = self.cells.get(&c) {
                    for &e in list {
                        if seen.insert(e) {
                            out.push(e);
                        }
                    }
                }
            }
        }

        out
    }
}

pub fn spatial_index_added(
    mut grid: ResMut<SpatialHashGrid>,
    q: Query<(Entity, &Transform), Added<ObjectKind>>,
) {
    for (e, t) in q.iter() {
        grid.insert_or_move(e, t.translation);
    }
}

pub fn spatial_index_changed(
    mut grid: ResMut<SpatialHashGrid>,
    q: Query<(Entity, &Transform), (With<ObjectKind>, Changed<Transform>)>,
) {
    for (e, t) in q.iter() {
        grid.insert_or_move(e, t.translation);
    }
}

pub fn spatial_index_removed(
    mut grid: ResMut<SpatialHashGrid>,
    mut removed: RemovedComponents<ObjectKind>,
) {
    for e in removed.read() {
        grid.remove(e);
    }
}
