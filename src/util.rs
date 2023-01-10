#[derive(Clone, Copy)]
pub struct Ray {
    pub pos: glam::Vec3,
    pub dir: glam::Vec3,
}

#[derive(Clone)]
pub struct Triangle {
    pub a: glam::Vec3,
    pub b: glam::Vec3,
    pub c: glam::Vec3,
}

pub type Frustum = [glam::Vec4; 6];
