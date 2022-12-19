use super::Ray;
use glam::Vec3;

pub struct World {}

impl World {
    pub fn collide(&self, ray: Ray) -> Option<Vec3> {
        let alpha = -ray.pos.y / ray.dir.y;
        match alpha > 0.0 {
            true => Some(ray.pos + alpha * ray.dir),
            false => None,
        }
    }
}
