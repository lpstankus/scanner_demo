use super::Ray;
use glam::{vec3, Vec3};
use noise::NoiseFn;

mod tables;

const SEED: u32 = 115;
const SCALE: f32 = 0.25;
const SURFACE_THRESHOLD: f64 = 0.5;

const MAX_RAY_DIST: u32 = 30;
const VOXEL_SIZE: f32 = 50.0;

#[derive(Debug)]
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

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

        for _ in 0..MAX_RAY_DIST {
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
    fn height(&self, pos: Vec3) -> f64 {
        let noise_pos = SCALE * pos;
        (self.noise.get([noise_pos.x as f64, noise_pos.y as f64, noise_pos.z as f64]) + 1.0) * 0.5
    }

    #[inline]
    fn voxel_collision(&self, voxel: Vec3, ray: Ray) -> Option<f32> {
        for triangle in self.voxel_triangles(voxel) {
            if let Some(t) = ray_triangle_collision(ray, triangle) {
                return Some(t);
            }
        }
        None
    }

    #[inline]
    fn voxel_triangles(&self, voxel: Vec3) -> Vec<Triangle> {
        let cube_indeces = [
            voxel + vec3(0.0, 0.0, 0.0),
            voxel + vec3(0.0, 0.0, 1.0),
            voxel + vec3(1.0, 0.0, 1.0),
            voxel + vec3(1.0, 0.0, 0.0),
            voxel + vec3(0.0, 1.0, 0.0),
            voxel + vec3(0.0, 1.0, 1.0),
            voxel + vec3(1.0, 1.0, 1.0),
            voxel + vec3(1.0, 1.0, 0.0),
        ];

        let mut cube_layout: usize = 0;
        for (i, vertex) in cube_indeces.iter().enumerate() {
            if self.height(vertex.clone()) < SURFACE_THRESHOLD {
                cube_layout |= 1 << i;
            }
        }

        let edges = tables::TRIANGULATION_TABLE[cube_layout];
        let mut triangles = Vec::with_capacity(5);

        let mut i = 0;
        while edges[i] != -1 {
            let a = edge_vertex(cube_indeces, edges[i + 0]);
            let b = edge_vertex(cube_indeces, edges[i + 1]);
            let c = edge_vertex(cube_indeces, edges[i + 2]);
            triangles.push(Triangle { a, b, c });
            i += 3;
        }

        triangles
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
fn ray_triangle_collision(ray: Ray, triangle: Triangle) -> Option<f32> {
    const EPSILON: f32 = 0.0001;

    let e1 = triangle.b - triangle.a;
    let e2 = triangle.c - triangle.a;

    let p = Vec3::cross(ray.dir, e2);
    let det = Vec3::dot(e1, p);
    if det.abs() < EPSILON {
        return None;
    }

    let inv_det = 1.0 / det;

    let tv = ray.pos - triangle.a;
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

#[inline]
fn edge_vertex(cube_vertices: [Vec3; 8], edge: i32) -> Vec3 {
    let (i1, i2) = tables::EDGE_TABLE[edge as usize];
    let a = cube_vertices[i1];
    let b = cube_vertices[i2];
    (Vec3::lerp(a, b, 0.5) - vec3(0.0, 1.0, 0.0)) * VOXEL_SIZE
}
