use super::util::{Ray, Triangle};
use glam::{vec3, Vec3};
use noise::NoiseFn;
use std::collections::HashMap;

mod tables;

const SEED: u32 = 115;
const SCALE: f32 = 0.01;
const SURFACE_THRESHOLD: f64 = 0.5;

const VOXEL_SIZE: f32 = 5.0;
const MAX_RAY_DIST: i32 = (1500.0 / VOXEL_SIZE) as i32;

type Voxel = (i32, i32, i32);

pub struct World {
    noise: noise::SuperSimplex,
    triangle_cache: HashMap<Voxel, Vec<Triangle>>,
}

impl World {
    pub fn new() -> Self {
        Self { noise: noise::SuperSimplex::new(SEED), triangle_cache: HashMap::new() }
    }

    pub fn retrieve_triangles(&mut self, center: Vec3, dist: f32) -> Vec<Triangle> {
        let mut tri_list = Vec::new();
        let base_voxel = (center / VOXEL_SIZE).floor();

        let off_dist = (dist / VOXEL_SIZE).ceil() as i32;
        for off in itertools::iproduct!(-off_dist..=off_dist, -off_dist..=off_dist, -off_dist..=off_dist) {
            let voxel = base_voxel + vec3(off.0 as f32, off.1 as f32, off.2 as f32);
            let mut triangles = self.voxel_triangles(voxel);
            tri_list.append(&mut triangles);
        }

        tri_list
    }

    pub fn raycast(&mut self, ray: Ray, dist: f32) -> Option<Vec3> {
        let mut cur_voxel = (ray.pos / VOXEL_SIZE).floor();

        if let Some(t_hit) = self.voxel_collision(cur_voxel, ray) {
            return handle_hit(ray, t_hit, dist);
        }

        let step = {
            let _step = |x: f32| (x < 0.0).then_some(-1.0).unwrap_or(1.0);
            vec3(_step(ray.dir.x), _step(ray.dir.y), _step(ray.dir.z))
        };

        let inv_dir = 1.0 / ray.dir;
        let mut t = {
            let min = (ray.pos / VOXEL_SIZE).floor() * VOXEL_SIZE;
            let max = min + VOXEL_SIZE;

            let t1 = (min - ray.pos) * inv_dir;
            let t2 = (max - ray.pos) * inv_dir;

            Vec3::max(t1, t2)
        };

        let delta_t = VOXEL_SIZE * inv_dir * step;
        let mut voxel_incr = Vec3::ZERO;

        let voxel_dist =
            if dist <= 0.0 { MAX_RAY_DIST } else { i32::max((dist / VOXEL_SIZE as f32).ceil() as i32, MAX_RAY_DIST) };

        for _ in 0..voxel_dist {
            voxel_incr.x = ((t.x <= t.y) && (t.x <= t.z)) as u32 as f32;
            voxel_incr.y = ((t.y <= t.x) && (t.y <= t.z)) as u32 as f32;
            voxel_incr.z = ((t.z <= t.x) && (t.z <= t.y)) as u32 as f32;

            t += voxel_incr * delta_t;
            cur_voxel += voxel_incr * step;

            if let Some(t_hit) = self.voxel_collision(cur_voxel, ray) {
                return handle_hit(ray, t_hit, dist);
            }
        }

        None
    }

    #[inline]
    fn voxel_collision(&mut self, voxel: Vec3, ray: Ray) -> Option<f32> {
        for triangle in self.voxel_triangles(voxel) {
            const EPSILON: f32 = 0.0001;

            let e1 = triangle.b - triangle.a;
            let e2 = triangle.c - triangle.a;

            let p = Vec3::cross(ray.dir, e2);
            let det = Vec3::dot(e1, p);
            if det.abs() < EPSILON {
                continue;
            }

            let inv_det = 1.0 / det;

            let tv = ray.pos - triangle.a;
            let u = Vec3::dot(tv, p) * inv_det;
            if u < 0.0 || u > 1.0 {
                continue;
            }

            let q = Vec3::cross(tv, e1);
            let v = Vec3::dot(ray.dir, q) * inv_det;
            if v < 0.0 || u + v > 1.0 {
                continue;
            }

            let t = Vec3::dot(e2, q) * inv_det;
            if t < EPSILON {
                continue;
            }

            return Some(t);
        }

        None
    }

    #[inline]
    fn voxel_triangles(&mut self, voxel: Vec3) -> Vec<Triangle> {
        let vx = (voxel.x as i32, voxel.y as i32, voxel.z as i32);
        if let Some(triangles) = self.triangle_cache.get(&vx) {
            return triangles.to_vec();
        }

        let mut cube_indeces = [
            (voxel + vec3(0.0, 0.0, 0.0), 0.0),
            (voxel + vec3(0.0, 0.0, 1.0), 0.0),
            (voxel + vec3(1.0, 0.0, 1.0), 0.0),
            (voxel + vec3(1.0, 0.0, 0.0), 0.0),
            (voxel + vec3(0.0, 1.0, 0.0), 0.0),
            (voxel + vec3(0.0, 1.0, 1.0), 0.0),
            (voxel + vec3(1.0, 1.0, 1.0), 0.0),
            (voxel + vec3(1.0, 1.0, 0.0), 0.0),
        ];

        let mut cube_layout: usize = 0;
        for (i, vertex) in cube_indeces.iter_mut().enumerate() {
            vertex.1 = self.surface_level(vertex.0);
            if vertex.1 < SURFACE_THRESHOLD {
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

        self.triangle_cache.insert(vx, triangles.to_vec());
        triangles
    }

    #[inline]
    fn surface_level(&self, pos: Vec3) -> f64 {
        let noise_pos = SCALE * VOXEL_SIZE * pos;
        (self.noise.get([noise_pos.x as f64, noise_pos.y as f64, noise_pos.z as f64]) + 1.0) * 0.5
    }
}

#[inline]
fn edge_vertex(cube_vertices: [(Vec3, f64); 8], edge: i32) -> Vec3 {
    let (i1, i2) = tables::EDGE_TABLE[edge as usize];
    let (a_voxel, a_val) = cube_vertices[i1];
    let (b_voxel, b_val) = cube_vertices[i2];
    let t = (SURFACE_THRESHOLD - a_val) / (b_val - a_val);
    Vec3::lerp(a_voxel, b_voxel, t as f32) * VOXEL_SIZE
}

#[inline]
fn handle_hit(ray: Ray, t: f32, dist: f32) -> Option<Vec3> {
    let hit_point = ray.pos + t * ray.dir;
    match dist <= 0.0 || Vec3::distance_squared(ray.pos, hit_point) <= dist * dist {
        true => Some(hit_point),
        false => None,
    }
}
