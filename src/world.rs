use super::Ray;
use glam::{vec3, Vec3};
use noise::NoiseFn;

const SEED: u32 = 115;
const SCALE: f64 = 0.005;

const VOXEL_SIZE: f32 = 10.0;
const HEIGHT: f32 = 5.0 * VOXEL_SIZE;

pub struct World {
    noise: noise::SuperSimplex,
}

impl World {
    pub fn new() -> Self {
        let noise = noise::SuperSimplex::new(SEED);
        Self { noise }
    }

    pub fn collide(&self, ray: Ray) -> Option<Vec3> {
        let mut cur_voxel = (ray.pos / VOXEL_SIZE).floor();

        let step = step_vector(ray.dir);
        let inv_dir = 1.0 / ray.dir;
        let mut t = calculate_t(ray.pos, inv_dir);

        if t.min_element() < 0.0 {
            panic!("{}\nMEU DEUS!!", t);
        }

        let delta_t = VOXEL_SIZE * inv_dir * step;
        let mut voxel_incr = Vec3::ZERO;

        while cur_voxel.y >= 0.0 {
            voxel_incr.x = ((t.x <= t.y) && (t.x <= t.z)) as u32 as f32;
            voxel_incr.y = ((t.y <= t.x) && (t.y <= t.z)) as u32 as f32;
            voxel_incr.z = ((t.z <= t.x) && (t.z <= t.y)) as u32 as f32;

            let old_t = t;
            t += voxel_incr * delta_t;
            cur_voxel += voxel_incr * step;
            match self.voxel_collision(cur_voxel, ray, old_t.min_element(), t.min_element()) {
                Some(t_hit) => return Some(ray.pos + t_hit * ray.dir),
                None => {}
            }
        }

        return Some(ray.pos + t.min_element() * ray.dir);
    }

    #[inline]
    fn height(&self, pos: Vec3) -> f32 {
        let noise = self.noise.get([pos.x as f64 * SCALE, pos.z as f64 * SCALE]) as f32;
        (noise + 1.0) / 2.0 * HEIGHT
    }

    #[inline]
    fn voxel_collision(&self, _voxel: Vec3, ray: Ray, t_min: f32, t_max: f32) -> Option<f32> {
        let mut t = t_min;
        while t <= t_max {
            let pos = ray.pos + t * ray.dir;
            if pos.y <= self.height(pos) {
                return Some(t);
            }
            t += 0.5;
        }
        None
    }
}

#[inline]
fn step_vector(dir: Vec3) -> Vec3 {
    let _step = |x: f32| (x < 0.0).then_some(-1.0).unwrap_or(1.0);
    vec3(_step(dir.x), _step(dir.y), _step(dir.z))
}

#[inline]
fn calculate_t(pos: Vec3, inv_dir: Vec3) -> Vec3 {
    let min = (pos / VOXEL_SIZE).floor() * VOXEL_SIZE;
    let max = min + VOXEL_SIZE;

    let t1 = (min - pos) * inv_dir;
    let t2 = (max - pos) * inv_dir;

    Vec3::max(t1, t2)
}
