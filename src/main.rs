use std::time::Instant;

use camera::Camera;
use marker::Marker;
use pollster::block_on;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use world::World;

mod camera;
mod marker;
pub mod util;
mod world;

const TITLE_UPDATE_TIME: f64 = 1.0;

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    camera: Camera,
    marker: Marker,
    world: World,

    title_timer: f64,
    title_update: bool,

    window: winit::window::Window,
}

impl State {
    fn new(window: winit::window::Window) -> State {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor { features: wgpu::Features::empty(), limits: wgpu::Limits::default(), label: None },
            None,
        ))
        .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };

        surface.configure(&device, &config);

        let camera = Camera::new(config.width as f32 / config.height as f32);
        let marker = Marker::new(&device, &config, &camera);
        let world = World::new();

        Self { surface, device, queue, config, camera, marker, world, title_timer: 0.0, title_update: false, window }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width as u32;
        self.config.height = height as u32;
        self.surface.configure(&self.device, &self.config);
    }

    fn update(&mut self, dt: f64) {
        self.update_camera(dt);
        self.update_marker(dt);

        self.title_timer -= dt;
        self.title_update = false;
        if self.title_timer <= 0.0 {
            self.title_timer += TITLE_UPDATE_TIME;
            self.title_update = true;
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            self.render_markers(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() -> Result<(), String> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    env_logger::init();
    let mut app_state = State::new(window);

    app_state.window.set_cursor_position(LogicalPosition { x: 0, y: 0 }).unwrap();
    app_state.window.set_cursor_grab(winit::window::CursorGrabMode::Confined).unwrap();
    app_state.window.set_cursor_visible(false);

    app_state.window.set_inner_size(LogicalSize { width: 1600, height: 900 });
    app_state.window.set_resizable(false);

    let mut now = Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent { ref event, .. } => device_event(&mut app_state, event),
        Event::WindowEvent { ref event, window_id } if window_id == app_state.window.id() => {
            window_event(&mut app_state, event, control_flow)
        }
        Event::RedrawRequested(window_id) if window_id == app_state.window.id() => {
            let dt = now.elapsed().as_secs_f64();
            now = Instant::now();

            app_state.update(dt);
            match app_state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    let size = app_state.window.inner_size();
                    app_state.resize(size.width, size.height);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => app_state.window.request_redraw(),
        _ => {}
    });
}

fn device_event(app_state: &mut State, event: &DeviceEvent) {
    match &event {
        DeviceEvent::MouseMotion { delta } => app_state.camera.offset_view(delta.0 as f32, delta.1 as f32),
        DeviceEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(_, y) } => {
            let delta = y * 0.0005;
            app_state.camera.ray_range = f32::clamp(app_state.camera.ray_range - delta, 0.1, 1.0);
        }
        _ => {}
    }
}

fn window_event(app_state: &mut State, event: &WindowEvent, control_flow: &mut ControlFlow) {
    match event {
        WindowEvent::CloseRequested
        | WindowEvent::KeyboardInput {
            input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. },
            ..
        } => *control_flow = ControlFlow::Exit,

        WindowEvent::Resized(size) => app_state.resize(size.width, size.height),
        WindowEvent::ScaleFactorChanged { new_inner_size: size, .. } => app_state.resize(size.width, size.height),

        WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
            app_state.marker.should_cast = state == &ElementState::Pressed
        }
        WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode, .. }, .. } => {
            let val = state == &ElementState::Pressed;
            if let Some(keycode) = virtual_keycode {
                match keycode {
                    VirtualKeyCode::W => app_state.camera.mov.forward = val,
                    VirtualKeyCode::S => app_state.camera.mov.backward = val,
                    VirtualKeyCode::A => app_state.camera.mov.left = val,
                    VirtualKeyCode::D => app_state.camera.mov.right = val,
                    VirtualKeyCode::Space => app_state.camera.mov.up = val,
                    VirtualKeyCode::LShift => app_state.camera.mov.down = val,
                    _ => {}
                }
            }
        }

        _ => {}
    }
}
