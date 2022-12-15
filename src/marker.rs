use super::camera::{Camera, CameraUniform};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

pub const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, 0.5] },
    Vertex { position: [-0.5, -0.5] },
    Vertex { position: [0.5, 0.5] },
    Vertex { position: [-0.5, -0.5] },
    Vertex { position: [0.5, -0.5] },
    Vertex { position: [0.5, 0.5] },
];

const INST_PER_ROW: usize = 10;
const INST_N: usize = INST_PER_ROW * INST_PER_ROW;
const INST_DISPLACEMENT: Vec3 = Vec3::new(INST_PER_ROW as f32 * 0.5, 0.5, INST_PER_ROW as f32 * 0.5);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct Mark {
    position: Vec3,
}

impl Mark {
    fn to_raw(&self) -> MarkRaw {
        let transform = Mat4::from_translation(self.position);
        MarkRaw { pos: self.position.into(), model: transform.to_cols_array_2d() }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MarkRaw {
    pos: [f32; 3],
    model: [[f32; 4]; 4],
}

impl MarkRaw {
    const ATTRIBS: [wgpu::VertexAttribute; 5] =
        wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct Marker {
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,

    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl Marker {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, camera: &Camera) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), MarkRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_data = Self::marks_data();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let camera_uniform = CameraUniform::new(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
            label: Some("camera_bind_group"),
        });

        Self { render_pipeline, vertex_buffer, instance_buffer, camera_uniform, camera_buffer, camera_bind_group }
    }

    fn marks_data() -> Vec<MarkRaw> {
        (0..INST_PER_ROW)
            .flat_map(|z| {
                (0..INST_PER_ROW).map(move |x| {
                    let position = glam::Vec3 { x: x as f32, y: 0.0, z: z as f32 } - INST_DISPLACEMENT;
                    Mark { position }.to_raw()
                })
            })
            .collect::<Vec<_>>()
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..INST_N as _);
    }
}
