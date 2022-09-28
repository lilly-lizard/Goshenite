use crate::shaders::shader_interfaces::{primitve_codes, PrimitiveDataUnit, PRIMITIVE_LEN};
use glam::Vec3;

/// bruh
type PrimitiveDataSlice = [PrimitiveDataUnit; PRIMITIVE_LEN];

#[derive(Default, Debug, Clone, Copy)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}
impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn encode(&self) -> PrimitiveDataSlice {
        [
            primitve_codes::SPHERE,
            self.center.x.to_bits(),
            self.center.y.to_bits(),
            self.center.z.to_bits(),
            self.radius.to_bits(),
            // padding
            primitve_codes::NULL,
            primitve_codes::NULL,
            primitve_codes::NULL,
        ]
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Cube {
    pub center: Vec3,
    pub dimensions: Vec3,
}
impl Cube {
    pub fn new(center: Vec3, dimensions: Vec3) -> Self {
        Self { center, dimensions }
    }

    pub fn encode(&self) -> PrimitiveDataSlice {
        [
            primitve_codes::CUBE,
            self.center.x.to_bits(),
            self.center.y.to_bits(),
            self.center.z.to_bits(),
            self.dimensions.x.to_bits(),
            self.dimensions.y.to_bits(),
            self.dimensions.z.to_bits(),
            // padding
            primitve_codes::NULL,
        ]
    }
}

#[derive(Default, Debug, Clone)]
pub struct Primitives {
    /// Encoded primitive data.
    data: Vec<PrimitiveDataUnit>,
    spheres: Vec<Sphere>,
}
impl Primitives {
    pub fn encoded_data(&self) -> &Vec<PrimitiveDataUnit> {
        &self.data
    }

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
        self.data.extend_from_slice(&sphere.encode());
    }

    pub fn spheres(&self) -> &Vec<Sphere> {
        &self.spheres
    }

    pub fn update_sphere(&mut self, index: usize, new_sphere: Sphere) {
        if let Some(s_ref) = self.spheres.get_mut(index) {
            let encoded = new_sphere.encode();
            let data_start = index * PRIMITIVE_LEN;
            let data_end = data_start + PRIMITIVE_LEN;
            self.data.splice(data_start..data_end, encoded);
            *s_ref = new_sphere;
        } else {
            todo!();
        }
    }
}
