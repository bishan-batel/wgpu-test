use std::{borrow::Cow, sync::Arc};

use encase::ShaderType;
use glam::{UVec3, Vec2, Vec4, uvec3, vec2};
use log::{error, info};
use wgpu::{
    BindGroupLayoutEntry, BufferUsages, CurrentSurfaceTexture,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{UnwrapThrowExt, prelude::*};

const VERTICES: &[Vec2] = &[
    vec2(-1., -1.),
    vec2(1., -1.),
    vec2(1., 1.),
    vec2(-1., 1.),
    //
];

const INDICES: &[UVec3] = &[
    uvec3(0, 1, 2),
    uvec3(0, 3, 2),
    //
];

#[repr(C)]
#[derive(Debug, ShaderType, bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Globals {
    color: Vec4,
}

#[derive(Debug)]
pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    globals: Globals,
    window: Arc<Window>,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    time: f32,
}

impl State {
    #[must_use = "Should not be creating a window just to drop it immediately"]
    pub async fn new(window: Arc<Window>) -> eyre::Result<Self> {
        let size = window.inner_size().max(winit::dpi::PhysicalSize {
            width: 1,
            height: 1,
        });

        // global WGPU instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            display: None,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create window surface");

        // adapter to the GPU, we are just using it to request a device & get a queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            // hard crash if we can't render anything
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,

                required_features: wgpu::Features::POLYGON_MODE_LINE,

                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },

                // value memory usage over raw performance
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                experimental_features: wgpu::ExperimentalFeatures::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("Surface is not supported by the given adapter");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.slang.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Global Bind Group"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);

        let Some(swapchain_format) = swapchain_capabilities.formats.first() else {
            eyre::bail!("Swapchain / Window Surface is incompatable with this device's adapter");
        };

        let swapchain_format = *swapchain_format; // dereference

        // create vertex buffer
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Triangle"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Triangle Index"),
            contents: bytemuck::cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });

        let globals = Globals {
            color: Vec4::new(0., 0., 1., 1.),
        };

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Globals UBO"),
            contents: bytemuck::bytes_of(&globals),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        info!("Created Buffers");

        use std::mem::size_of;

        let render_pipeline: wgpu::RenderPipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default Triangle Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: (2 * size_of::<f32>()) as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    }],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    // targets for the fragment shader
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            globals,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            render_pipeline,
            bind_group_layout,
            is_surface_configured: false,
            time: 0.,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.is_surface_configured = true;
    }

    pub fn render(&mut self) -> eyre::Result<()> {
        if !self.is_surface_configured {
            return Ok(());
        }

        self.time += 0.01;
        self.globals.color.x = self.time.cos().powi(2);
        self.globals.color.y = self.time.sin().powi(2);

        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Timeout
            | CurrentSurfaceTexture::Occluded
            | CurrentSurfaceTexture::Validation
            | CurrentSurfaceTexture::Outdated => return Ok(()), // skip this frame
            CurrentSurfaceTexture::Lost => eyre::bail!("Lost WGPU Device"),
        };

        // view into the texture
        let frame_tex_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.queue.write_buffer(&self.uniform_buffer, 0, &{
            let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
            buffer.write(&self.globals).unwrap();
            buffer.into_inner()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Triangle Enconder"),
            });

        let ubo_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    self.uniform_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        // begin render pass(es)
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_tex_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.1,
                            b: 0.1,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_bind_group(0, Some(&ubo_bind_group), &[]);
            rpass.set_pipeline(&self.render_pipeline);

            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(0..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            rpass.draw_indexed(0..(3 * INDICES.len() as u32), 0, 0..1);
        }

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(self.queue.submit([encoder.finish()])),
                timeout: None,
            })
            .expect("Failed to poll");

        frame.present();
        Ok(())
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &winit::event_loop::EventLoop<State>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());

        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            window: None,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the
            // self.state = Some(pollster::block_on(State::new(window)).unwrap());
            self.window = Some(window);
        }

        info!("Resumed");

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        // state.window.request_redraw();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if let Err(report) = state.render() {
                    error!("RedrawRequested Error while rendering: {report}");
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
            },
            _ => {}
        }
    }

    // ...
}
