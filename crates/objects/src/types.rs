use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectTypeId(pub u16);

#[derive(Clone, Debug)]
pub struct ObjectTypeSpec {
    pub name: String,
    /// Path relative to the Bevy asset root (the `assets/` folder).
    pub gltf: String,
    pub gltf_bounds: Option<GltfBounds>,
    pub render_scale: Vec3,
    pub hover_radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct GltfBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl GltfBounds {
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}

#[derive(Default)]
pub struct ObjectTypeRegistry {
    specs: Vec<Option<ObjectTypeSpec>>,
    free_list: Vec<u16>,
}

impl ObjectTypeRegistry {
    pub fn register(&mut self, spec: ObjectTypeSpec) -> ObjectTypeId {
        if let Some(id) = self.free_list.pop() {
            self.specs[id as usize] = Some(spec);
            return ObjectTypeId(id);
        }

        let id = self.specs.len() as u16;
        self.specs.push(Some(spec));
        ObjectTypeId(id)
    }

    pub fn get(&self, id: ObjectTypeId) -> Option<&ObjectTypeSpec> {
        self.specs.get(id.0 as usize)?.as_ref()
    }
}
