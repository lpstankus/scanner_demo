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

pub const TO_WGPU_MATRIX: Mat4 = glam::mat4(
    glam::vec4(1.0, 0.0, 0.0, 0.0),
    glam::vec4(0.0, 1.0, 0.0, 0.0),
    glam::vec4(0.0, 0.0, 0.5, 0.0),
    glam::vec4(0.0, 0.0, 0.5, 1.0),
);

const CAM_SENSITIVITY: f32 = 0.001;
const MOV_SENSITIVITY: f32 = 7.5;

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,

            yaw: std::f32::consts::PI,
            pitch: 0.0,

            pos: (0.0, 1.0, 2.0).into(),
            dir: (-1.0, 0.0, 0.0).into(),
            up: (0.0, 1.0, 0.0).into(),

            mov: Movement { forward: false, backward: false, right: false, left: false },
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_to_rh(self.pos, self.dir, self.up);
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        return TO_WGPU_MATRIX * proj * view;
    }

    pub fn update(&mut self, dt: f64) {
        self.pos += self.movement_dir() * Vec3::splat(dt as f32) * MOV_SENSITIVITY;
    }

    fn movement_dir(&self) -> Vec3 {
        let mut movement_dir = Vec3::splat(0.0);
        self.mov.forward.then(|| movement_dir += self.dir);
        self.mov.backward.then(|| movement_dir -= self.dir);
        self.mov.right.then(|| movement_dir += Vec3::cross(self.dir, self.up));
        self.mov.left.then(|| movement_dir -= Vec3::cross(self.dir, self.up));
        movement_dir
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
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self { view_proj: camera.build_view_projection_matrix().to_cols_array_2d() }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}