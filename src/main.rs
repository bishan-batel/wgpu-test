extern crate nalgebra_glm as glm;

use std::borrow::Cow;
use wgpu::{
    BindGroupDescriptor, BindGroupLayoutEntry, BufferUsages, CurrentSurfaceTexture,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use glm::{Vec2, Vec4};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

#[derive(Debug)]
pub struct App {}

async fn run(event_loop: EventLoop<()>, window: Window) {
    // window / framebuffer size
    let size = window.inner_size().max(PhysicalSize {
        width: 1,
        height: 1,
    });

    // global WGPU instance
    let instance = wgpu::Instance::default();

    let surface = instance
        .create_surface(&window)
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

            required_features: wgpu::Features::empty(),

            // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
            required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                .using_resolution(adapter.limits()),

            // value memory usage over raw performance
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            experimental_features: wgpu::ExperimentalFeatures::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Failed to create device");

    let mut surface_config = surface
        .get_default_config(&adapter, size.width, size.height)
        .expect("Surface is not supported by the given adapter");

    surface.configure(&device, &surface_config);

    // Load the shader modules (wglsl)
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.wgsl"))),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Global Bind Group"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::all(),
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

    let swapchain_format = *swapchain_capabilities
        .formats
        .first()
        .expect("Incompatable adapter for this surface");

    const VERTICES: &[Vec2] = &[
        Vec2::new(0.1, 0.1), //
        Vec2::new(0.4, 0.2), //
        Vec2::new(0.2, 0.4), //
    ];

    // create vertex buffer
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Triangle"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: BufferUsages::VERTEX,
    });

    #[repr(C, align(4))]
    #[derive(Debug, bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
    struct Globals {
        color: Vec4,
    }

    let mut globals = Globals {
        color: Vec4::new(0., 0., 1., 1.),
    };

    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Globals UBO"),
        contents: bytemuck::bytes_of(&globals),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let ubo_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
        }],
    });

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
            //
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

    let window = &window;

    let mut time = 0.0f32;

    event_loop
        .run(move |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            let _ = (&instance, &adapter, &shader, &pipeline_layout);

            window.request_redraw();

            if let Event::WindowEvent {
                window_id: _,
                event,
            } = event
            {
                match event {
                    WindowEvent::Resized(new_size) => {
                        // Reconfigure the surface with the new size
                        surface_config.width = new_size.width.max(1);
                        surface_config.height = new_size.height.max(1);
                        surface.configure(&device, &surface_config);

                        // On macos the window needs to be redrawn manually after resizing
                        window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        // get the current surface texture, if we can't get it then just dip this
                        // frame
                        let frame = match surface.get_current_texture() {
                            CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
                            CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
                            _ => return,
                        };

                        // view into the texture
                        let frame_tex_view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        // update color in a sin-fasion
                        time += 0.01;
                        globals.color.x = time.cos().powi(2);
                        globals.color.y = time.sin().powi(2);

                        // update globals into the uniform buffer
                        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&globals));
                        queue.submit([]);

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Triangle Enconder"),
                            });

                        // begin render pass(es)
                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                            rpass.set_pipeline(&render_pipeline);
                            rpass.set_vertex_buffer(0, vertex_buffer.slice(0..));

                            rpass.draw(0..(VERTICES.len() as u32), 0..1);
                        }

                        device
                            .poll(wgpu::PollType::Wait {
                                submission_index: Some(queue.submit([encoder.finish()])),
                                timeout: None,
                            })
                            .expect("Failed to poll");

                        frame.present();
                    }
                    //
                    WindowEvent::CloseRequested => target.exit(),
                    _ => {}
                };
            }
        })
        .expect("Unhandled error ocurred when running event loop");
}

pub fn main() -> eyre::Result<()> {
    // colored panics
    color_eyre::install()?;

    tracing_subscriber::fmt().init();

    let event_loop = EventLoop::new()?;

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .with_inner_size(PhysicalSize::new(800, 800))
        .build(&event_loop)?;

    pollster::block_on(run(event_loop, window));

    Ok(())
}
