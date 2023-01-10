use std::ops::{Deref, Index};

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

#[derive(Debug)]
pub struct SVec<T, const N: usize> {
    len: usize,
    buf: [T; N],
}

impl<T, const N: usize> SVec<T, N> {
    pub fn new() -> Self {
        Self { len: 0, buf: unsafe { std::mem::MaybeUninit::uninit().assume_init() } }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push(&mut self, value: T) -> bool {
        if self.len >= N {
            return false;
        }
        self.buf[self.len] = value;
        self.len += 1;
        true
    }
}

impl<T, const N: usize> Index<usize> for SVec<T, N> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.buf[index]
    }
}

impl<T, const N: usize> Deref for SVec<T, N> {
    type Target = [T];

    fn deref<'a>(&'a self) -> &'a [T] {
        &self.buf[0..self.len]
    }
}
