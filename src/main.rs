use camera::Camera;
use marker::Marker;
use pollster::block_on;
use sdl2::{event::Event, event::WindowEvent, keyboard::Keycode, mouse::MouseButton, mouse::MouseWheelDirection};
use world::World;

mod camera;
mod marker;
mod world;

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

    window: sdl2::video::Window,
}

impl State {
    fn new(window: sdl2::video::Window) -> State {
        let size = window.drawable_size();

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
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Immediate,
        };

        surface.configure(&device, &config);

        let camera = Camera::new(config.width as f32 / config.height as f32);
        let marker = Marker::new(&device, &config, &camera);
        let world = World::new();

        Self { surface, device, queue, config, camera, marker, world, title_timer: 0.0, title_update: false, window }
    }

    fn resize(&mut self, width: i32, height: i32) {
        if width > 0 && height > 0 {
            self.config.width = width as u32;
            self.config.height = height as u32;
            self.surface.configure(&self.device, &self.config);
        }
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
    let sdl = sdl2::init()?;
    let timer = sdl.timer()?;
    let video = sdl.video()?;
    let window = video
        .window("Scanner Demo", 1280, 720)
        .position_centered()
        .input_grabbed()
        .build()
        .map_err(|e| e.to_string())?;

    sdl.mouse().show_cursor(true);
    sdl.mouse().set_relative_mouse_mode(true);

    env_logger::init();
    let mut state = State::new(window);

    let mut cur = timer.performance_counter();
    let mut prev = cur;
    loop {
        cur = timer.performance_counter();
        let dt = (cur - prev) as f64 / timer.performance_frequency() as f64;
        prev = cur;

        handle_events(&sdl, &mut state);
        state.update(dt);
        match state.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => {
                let size = state.window.size();
                state.resize(size.0 as i32, size.1 as i32);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => std::process::exit(-1),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn handle_events(sdl: &sdl2::Sdl, state: &mut State) {
    for event in sdl.event_pump().unwrap().poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => std::process::exit(0),
            Event::Window { win_event: WindowEvent::Resized(width, height), .. } => state.resize(width, height),
            Event::MouseMotion { xrel, yrel, .. } => handle_mousemotion(xrel as f32, yrel as f32, state),
            Event::KeyDown { keycode, .. } => handle_keydown(keycode.unwrap(), state),
            Event::KeyUp { keycode, .. } => handle_keyup(keycode.unwrap(), state),
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, .. } => state.marker.should_cast = true,
            Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => state.marker.should_cast = false,
            Event::MouseWheel { y, direction: MouseWheelDirection::Normal, .. } => handle_mousewheel(y, state),
            _ => {}
        }
    }
}

fn handle_mousewheel(y: i32, state: &mut State) {
    state.camera.ray_range = f32::clamp(state.camera.ray_range + y as f32 * 0.1, 0.1, 1.0);
}

fn handle_mousemotion(xrel: f32, yrel: f32, state: &mut State) {
    state.camera.offset_view(xrel, yrel);
}

fn handle_keydown(keycode: Keycode, state: &mut State) {
    match keycode {
        Keycode::W => state.camera.mov.forward = true,
        Keycode::S => state.camera.mov.backward = true,
        Keycode::D => state.camera.mov.right = true,
        Keycode::A => state.camera.mov.left = true,
        Keycode::Space => state.camera.mov.up = true,
        Keycode::LShift => state.camera.mov.down = true,
        _ => {}
    }
}

fn handle_keyup(keycode: Keycode, state: &mut State) {
    match keycode {
        Keycode::W => state.camera.mov.forward = false,
        Keycode::S => state.camera.mov.backward = false,
        Keycode::D => state.camera.mov.right = false,
        Keycode::A => state.camera.mov.left = false,
        Keycode::Space => state.camera.mov.up = false,
        Keycode::LShift => state.camera.mov.down = false,
        _ => {}
    }
}
