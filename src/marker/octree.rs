use super::super::Frustum;
use super::{Mark, MarkRaw};
use glam::{vec3, Vec3};

const BASE_EXTENSION: f32 = 50.0;

pub struct Octree {
    root: u32,
    bucket_size: u32,
    octants: Vec<Octant>,
}

impl Octree {
    pub fn new(bucket_size: u32) -> Self {
        Self {
            root: 0,
            bucket_size,
            octants: vec![Octant {
                center: vec3(0.0, 0.0, 0.0),
                extension: BASE_EXTENSION,
                content: Content::Leaf(Vec::with_capacity(bucket_size as usize)),
            }],
        }
    }

    pub fn insert(&mut self, mark: Mark) {
        if !self[self.root].contains(mark) {
            return;
        }

        let mark = mark.to_raw();
        let bucket_size = self.bucket_size as usize;
        let mut id = self.root;
        loop {
            let center = self[id].center;
            let mut children = match self[id].content {
                Content::Parent(children) => {
                    let mut child_id = 0;
                    for i in 0..3 {
                        if mark.pos[i] > self[id].center[i] {
                            child_id |= 1 << i;
                        }
                    }
                    id = children[child_id];
                    continue;
                }
                Content::Leaf(ref mut data) => {
                    if data.len() < bucket_size {
                        data.push(mark);
                        return;
                    }

                    let mut children = Vec::with_capacity(8);
                    let mut children_data = Vec::with_capacity(8);
                    for _ in 0..8 {
                        children_data.push(Vec::with_capacity(bucket_size));
                    }

                    for mark in data {
                        let mut child_id = 0;
                        for i in 0..3 {
                            if mark.pos[i] > center[i] {
                                child_id |= 1 << i;
                            }
                        }
                        children_data[child_id].push(mark.clone());
                    }

                    for i in 0..8 {
                        let mut offset = Vec3::ZERO;
                        for j in 0..3 {
                            if (7 - i) & 1 << j != 0 {
                                offset[j] = self[id].extension / 2.0;
                            } else {
                                offset[j] = -self[id].extension / 2.0;
                            }
                        }
                        children.push(Octant {
                            center: self[id].center + offset,
                            extension: self[id].extension / 2.0,
                            content: Content::Leaf(children_data.pop().unwrap()),
                        });
                    }

                    children
                }
            };

            let mut children_ids = [0; 8];
            for i in 0..8 {
                children_ids[i] = self.octants.len() as u32;
                self.octants.push(children.pop().unwrap());
            }
            self[id].content = Content::Parent(children_ids);
        }
    }

    pub fn count(&self) -> usize {
        let mut sum = 0;
        for oct in &self.octants {
            if let Content::Leaf(ref data) = oct.content {
                sum += data.len();
            }
        }
        sum
    }

    pub fn get_visible(&mut self, vec: &mut Vec<MarkRaw>, pos: Vec3, frustum: Frustum) {
        vec.truncate(0);
        self.get_visible_rec(vec, self.root, pos, frustum);
    }

    fn get_visible_rec(&mut self, vec: &mut Vec<MarkRaw>, id: u32, pos: Vec3, frustum: Frustum) {
        if vec.len() == vec.capacity() {
            return;
        }

        match self[id].content {
            Content::Parent(children) => {
                let mut children = children.clone();
                children.sort_unstable_by(|a, b| {
                    let dist_a = Vec3::distance_squared(self[*a].center, pos);
                    let dist_b = Vec3::distance_squared(self[*b].center, pos);
                    f32::total_cmp(&dist_b, &dist_a)
                });
                'outer: for child_id in children {
                    for plane in frustum {
                        if !self[child_id].collide(plane) {
                            continue 'outer;
                        }
                    }
                    self.get_visible_rec(vec, child_id, pos, frustum);
                }
            }
            Content::Leaf(ref data) => {
                let end = usize::min(vec.capacity() - vec.len(), data.len());
                vec.extend(&data[..end]);
            }
        }
    }
}

impl std::ops::Index<u32> for Octree {
    type Output = Octant;
    fn index(&self, index: u32) -> &Self::Output {
        &self.octants[index as usize]
    }
}

impl std::ops::IndexMut<u32> for Octree {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.octants[index as usize]
    }
}

#[derive(Debug)]
enum Content {
    Parent([u32; 8]),
    Leaf(Vec<MarkRaw>),
}

#[derive(Debug)]
pub struct Octant {
    center: Vec3,
    extension: f32,
    content: Content,
}

impl Octant {
    #[inline]
    fn contains(&self, mark: Mark) -> bool {
        let under = mark.pos.x < self.center.x - self.extension
            || mark.pos.y < self.center.y - self.extension
            || mark.pos.z < self.center.z - self.extension;

        let above = mark.pos.x >= self.center.x + self.extension
            || mark.pos.y >= self.center.y + self.extension
            || mark.pos.z >= self.center.z + self.extension;

        !(above || under)
    }

    #[inline]
    fn collide(&self, plane: glam::Vec4) -> bool {
        let r = self.extension * (plane.x.abs() + plane.y.abs() + plane.z.abs());
        let s = Vec3::dot(plane.truncate(), self.center) - plane.w;
        -r <= s
    }
}
