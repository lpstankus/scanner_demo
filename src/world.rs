use super::Ray;
use glam::Vec3;
use noise::NoiseFn;

const SEED: u32 = 115;
const HEIGHT: f32 = 100.0;
const SCALE: f64 = 0.005;

const STEP: f32 = 0.5;

pub struct World {
    noise: noise::SuperSimplex,
}

impl World {
    pub fn new() -> Self {
        let noise = noise::SuperSimplex::new(SEED);
        Self { noise }
    }

    pub fn collide(&self, ray: Ray) -> Option<Vec3> {
        if (ray.dir.y == 0.0) || (ray.dir.y < 0.0 && ray.pos.y < 0.0) || (ray.dir.y > 0.0 && ray.pos.y > HEIGHT) {
            return None;
        }

        match ray.dir.y < 0.0 {
            false => self.cast_up(ray),
            true => self.cast_down(ray),
        }
    }

    fn cast_down(&self, ray: Ray) -> Option<Vec3> {
        let mut t = -(ray.pos.y - HEIGHT) / ray.dir.y;
        let mut pos = ray.pos + t * ray.dir;
        while pos.y > 0.0 {
            let terrain_height = self.height(pos);
            if terrain_height >= pos.y {
                return Some(pos);
            }
            t += STEP;
            pos = ray.pos + t * ray.dir;
        }
        None
    }

    fn cast_up(&self, ray: Ray) -> Option<Vec3> {
        let mut t = -ray.pos.y / ray.dir.y;
        let mut pos = ray.pos + t * ray.dir;
        while pos.y < HEIGHT {
            let terrain_height = self.height(pos);
            if terrain_height <= pos.y {
                return Some(pos);
            }
            t += STEP;
            pos = ray.pos + t * ray.dir;
        }
        None
    }

    fn height(&self, pos: Vec3) -> f32 {
        let noise = self.noise.get([pos.x as f64 * SCALE, pos.z as f64 * SCALE]) as f32;
        (noise + 1.0) / 2.0 * HEIGHT
    }
}
