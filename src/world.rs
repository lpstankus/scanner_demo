use super::Ray;
use glam::{vec3, Vec3};
use noise::NoiseFn;

const SEED: u32 = 115;
const SCALE: f64 = 0.005;

const VOXEL_SIZE: f32 = 50.0;
const HEIGHT: f32 = 5.0 * VOXEL_SIZE;

const MAX_VOXEL_DIST: u32 = 1000;

pub struct World {
    noise: noise::SuperSimplex,
}

impl World {
    pub fn new() -> Self {
        let noise = noise::SuperSimplex::new(SEED);
        Self { noise }
    }

    pub fn collide(&self, ray: Ray) -> Option<Vec3> {
        let mut cur_voxel = (ray.pos / VOXEL_SIZE).floor() + vec3(0.0, 1.0, 0.0);

        if let Some(t_hit) = self.voxel_collision(cur_voxel, ray) {
            return Some(ray.pos + t_hit * ray.dir);
        }

        let step = step_vector(ray.dir);
        let inv_dir = 1.0 / ray.dir;
        let mut t = calculate_t(ray.pos, inv_dir);

        let delta_t = VOXEL_SIZE * inv_dir * step;
        let mut voxel_incr = Vec3::ZERO;

        for _ in 0..MAX_VOXEL_DIST {
            voxel_incr.x = ((t.x <= t.y) && (t.x <= t.z)) as u32 as f32;
            voxel_incr.y = ((t.y <= t.x) && (t.y <= t.z)) as u32 as f32;
            voxel_incr.z = ((t.z <= t.x) && (t.z <= t.y)) as u32 as f32;

            t += voxel_incr * delta_t;
            cur_voxel += voxel_incr * step;

            if let Some(t_hit) = self.voxel_collision(cur_voxel, ray) {
                return Some(ray.pos + t_hit * ray.dir);
            }
        }

        None
    }

    #[inline]
    fn height(&self, pos: Vec3) -> f32 {
        let noise = self.noise.get([pos.x as f64 * SCALE, pos.z as f64 * SCALE]) as f32;
        (noise + 1.0) / 2.0 * HEIGHT
    }

    #[inline]
    fn voxel_collision(&self, voxel: Vec3, ray: Ray) -> Option<f32> {
        let a = (voxel + vec3(0.5, 0.0, 0.0)) * VOXEL_SIZE;
        let b = (voxel + vec3(1.0, -1.0, 1.0)) * VOXEL_SIZE;
        let c = (voxel + vec3(0.0, -1.0, 1.0)) * VOXEL_SIZE;
        ray_triangle_collision(ray, a, b, c)
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

#[inline]
fn ray_triangle_collision(ray: Ray, p1: Vec3, p2: Vec3, p3: Vec3) -> Option<f32> {
    const EPSILON: f32 = 0.0001;

    let e1 = p2 - p1;
    let e2 = p3 - p1;

    let p = Vec3::cross(ray.dir, e2);
    let det = Vec3::dot(e1, p);
    if det.abs() < EPSILON {
        return None;
    }

    let inv_det = 1.0 / det;

    let tv = ray.pos - p1;
    let u = Vec3::dot(tv, p) * inv_det;
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = Vec3::cross(tv, e1);
    let v = Vec3::dot(ray.dir, q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = Vec3::dot(e2, q) * inv_det;
    if t < EPSILON {
        return None;
    }

    Some(t)
}
