use std::borrow::Cow;

use glm::Vec2;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages,
};
use winit::{event::WindowEvent, window::Window};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vertex {
    position: Vec2,
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: Vec2::new(-1., 0.),
    },
    Vertex {
        position: Vec2::new(0., 1.),
    },
    Vertex {
        position: Vec2::new(1., 0.),
    },
];

pub struct Engine<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    shader: wgpu::ShaderModule,

    window: &'a Window,
}

impl<'a> Engine<'a> {
    pub async fn new(window: &'a Window) -> Self {
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(&window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,

                    required_features: wgpu::Features::empty(),

                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),

                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        surface.configure(&device, &surface_config);

        Self {
            // vertex_buffer,
            // adapter,
            // pipeline_layout,
            surface,
            device,
            queue,
            surface_config,
            // boid_render_pipeline,
            window,
            size,
        }
    }

    fn process(&mut self) {}

    fn render(&mut self) {}

    pub fn event(&mut self, event: &WindowEvent) {}
}
