use super::Ray;
use glam::{Mat4, Vec3};

#[repr(packed)]
pub struct Movement {
    pub forward: bool,
    pub backward: bool,
    pub right: bool,
    pub left: bool,
}

pub struct Camera {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,

    yaw: f32,
    pitch: f32,

    pos: Vec3,
    dir: Vec3,
    up: Vec3,

    pub mov: Movement,
}

const TO_WGPU_MATRIX: Mat4 = glam::mat4(
    glam::vec4(1.0, 0.0, 0.0, 0.0),
    glam::vec4(0.0, 1.0, 0.0, 0.0),
    glam::vec4(0.0, 0.0, 0.5, 0.0),
    glam::vec4(0.0, 0.0, 0.5, 1.0),
);

const PI: f32 = std::f32::consts::PI;

const CAM_SENSITIVITY: f32 = 0.001;
const MOV_SENSITIVITY: f32 = 20.0;

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,

            yaw: PI,
            pitch: (-89.0 as f32).to_radians(),

            pos: (0.0, 30.0, 0.0).into(),
            dir: (-1.0, 0.0, 0.0).into(),
            up: (0.0, 1.0, 0.0).into(),

            mov: Movement { forward: false, backward: false, right: false, left: false },
        }
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.pos, self.dir, self.up)
    }

    fn projection_matrix(&self) -> Mat4 {
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        return TO_WGPU_MATRIX * proj;
    }

    pub fn update(&mut self, dt: f64) {
        self.pos += self.movement_dir() * Vec3::splat(dt as f32) * MOV_SENSITIVITY;
    }

    fn movement_dir(&self) -> Vec3 {
        let mut movement_dir = Vec3::splat(0.0);
        self.mov.forward.then(|| movement_dir += self.dir);
        self.mov.backward.then(|| movement_dir -= self.dir);
        self.mov.right.then(|| movement_dir += Vec3::cross(self.dir, self.up).normalize());
        self.mov.left.then(|| movement_dir -= Vec3::cross(self.dir, self.up).normalize());
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
        let length = rand::random::<f32>() * 0.25;

        let right = Vec3::cross(self.dir, self.up).normalize();
        let up = Vec3::cross(self.dir, right).normalize();

        let offset = length * ((right * f32::sin(angle)) + (up * f32::cos(angle)));
        Ray { pos: self.pos, dir: (self.dir + offset).normalize() }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
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
