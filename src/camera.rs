use super::{Frustum, Ray, State, Triangle};
use glam::{vec3, Mat4, Vec3, Vec4Swizzles};

#[repr(packed)]
pub struct Movement {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
}

pub struct Camera {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,

    yaw: f32,
    pitch: f32,

    pub pos: Vec3,
    dir: Vec3,
    up: Vec3,

    pub ray_range: f32,
    pub mov: Movement,
}

const TO_WGPU_MATRIX: Mat4 = glam::mat4(
    glam::vec4(1.0, 0.0, 0.0, 0.0),
    glam::vec4(0.0, 1.0, 0.0, 0.0),
    glam::vec4(0.0, 0.0, 0.5, 0.0),
    glam::vec4(0.0, 0.0, 0.5, 1.0),
);

const PI: f32 = std::f32::consts::PI;

const N_ITERATIONS: i32 = 5;

const CAM_SIZE: f32 = 1.0;
const CAM_SENSITIVITY: f32 = 0.001;
const MOV_SPEED: f32 = 100.0;

impl Camera {
    pub fn new(aspect: f32) -> Self {
        let yaw = -PI / 2.0;
        let pitch = 0.0;
        let dir = Vec3 { x: f32::cos(yaw) * f32::cos(pitch), y: f32::sin(pitch), z: f32::sin(yaw) * f32::cos(pitch) };

        Self {
            aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000000.0,
            yaw,
            pitch,
            pos: vec3(0.0, 0.0, -30.0),
            dir,
            up: vec3(0.0, 1.0, 0.0),
            ray_range: 0.5,
            mov: Movement { forward: false, backward: false, right: false, left: false, up: false, down: false },
        }
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.pos, self.dir, self.up)
    }

    fn projection_matrix(&self) -> Mat4 {
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        return TO_WGPU_MATRIX * proj;
    }

    fn movement_dir(&self) -> Vec3 {
        let right = Vec3::cross(self.dir, self.up).normalize();
        let up = Vec3::cross(right, self.dir).normalize();

        let mut movement_dir = Vec3::splat(0.0);
        self.mov.forward.then(|| movement_dir += self.dir);
        self.mov.backward.then(|| movement_dir -= self.dir);
        self.mov.right.then(|| movement_dir += right);
        self.mov.left.then(|| movement_dir -= right);
        self.mov.up.then(|| movement_dir += up);
        self.mov.down.then(|| movement_dir -= up);
        movement_dir.normalize_or_zero()
    }

    pub fn offset_view(&mut self, xrel: f32, yrel: f32) {
        self.yaw += xrel * CAM_SENSITIVITY;
        self.pitch -= yrel * CAM_SENSITIVITY;
        self.pitch = self.pitch.clamp((-89.0 as f32).to_radians(), (89.0 as f32).to_radians());

        let dir = Vec3 {
            x: f32::cos(self.yaw) * f32::cos(self.pitch),
            y: f32::sin(self.pitch),
            z: f32::sin(self.yaw) * f32::cos(self.pitch),
        };
        self.dir = dir.normalize();
    }

    pub fn cast_ray(&self) -> Ray {
        let angle = rand::random::<f32>() * 2.0 * PI;
        let length = rand::random::<f32>() * self.ray_range * 0.5;

        let right = Vec3::cross(self.dir, self.up).normalize();
        let up = Vec3::cross(self.dir, right).normalize();

        let offset = length * ((right * f32::sin(angle)) + (up * f32::cos(angle)));
        Ray { pos: self.pos, dir: (self.dir + offset).normalize() }
    }

    pub fn frustum(&self) -> Frustum {
        let to_plane = |vec: glam::Vec4| vec.xyz().extend(-vec.w);
        let mat = self.projection_matrix() * self.view_matrix();
        [
            to_plane(mat.row(3) + mat.row(0)), // left
            to_plane(mat.row(3) - mat.row(0)), // right
            to_plane(mat.row(3) + mat.row(1)), // bottom
            to_plane(mat.row(3) - mat.row(1)), // top
            to_plane(mat.row(2)),              // near
            to_plane(mat.row(3) - mat.row(2)),
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pos: [f32; 3],
    padding: f32,
    to_view: [[f32; 4]; 4],
    to_clip: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            pos: camera.pos.into(),
            padding: 0.0,
            to_view: camera.view_matrix().to_cols_array_2d(),
            to_clip: camera.projection_matrix().to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.pos = camera.pos.into();
        self.to_view = camera.view_matrix().to_cols_array_2d();
        self.to_clip = camera.projection_matrix().to_cols_array_2d();
    }
}

impl State {
    pub fn update_camera(&mut self, dt: f64) {
        self.camera.pos += self.camera.movement_dir() * MOV_SPEED * dt as f32;

        let triangle_list = self.world.retrieve_triangles(self.camera.pos, CAM_SIZE);
        for _ in 0..N_ITERATIONS {
            let mut inf_dir = Vec3::ZERO;
            for triangle in &triangle_list {
                if let Some(dir) = self.collide_camera(triangle.clone()) {
                    inf_dir += dir;
                }
            }
            if inf_dir != Vec3::ZERO {
                self.camera.pos += inf_dir;
                break;
            }
        }

        self.marker.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.marker.camera_buffer, 0, bytemuck::cast_slice(&[self.marker.camera_uniform]));
    }

    #[inline]
    fn collide_camera(&mut self, triangle: Triangle) -> Option<Vec3> {
        let e1 = triangle.b - triangle.a;
        let e2 = triangle.c - triangle.a;

        let n = Vec3::cross(e1, e2).normalize();
        let dist = Vec3::dot(self.camera.pos - triangle.a, n);
        let p = self.camera.pos - dist * n;

        if dist <= CAM_SIZE && point_in_triangle(p, triangle) {
            Some((CAM_SIZE - dist) * n)
        } else {
            None
        }
    }
}

#[inline]
fn point_in_triangle(p: Vec3, t: Triangle) -> bool {
    let sign = |p1: Vec3, p2: Vec3, p3: Vec3| (p1.x - p3.x) * (p2.y - p3.y) * (p1.y - p3.y);

    let d1 = sign(p, t.a, t.b);
    let d2 = sign(p, t.b, t.c);
    let d3 = sign(p, t.c, t.a);

    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

    !(has_neg && has_pos)
}
