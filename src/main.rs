use encase::ShaderType;
use log::{error, info};
use std::borrow::Cow;
use wgpu::{
    BindGroupDescriptor, BindGroupLayoutEntry, BufferUsages, CurrentSurfaceTexture,
    RenderPassColorAttachment, TextureDescriptor, TextureUsages, TextureViewDescriptor,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use glam::{UVec3, Vec2, Vec4};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

// #[repr(C)]
// #[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
#[derive(ShaderType)]
struct Globals {
    color: Vec4,
}

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

            required_features: wgpu::Features::POLYGON_MODE_LINE,

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
        // source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.wgsl"))),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.slang.wgsl"))),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Global Bind Group"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);

    let Some(swapchain_format) = swapchain_capabilities.formats.first() else {
        error!("Swapchain / Window Surface is incompatable with this device's adapter");
        return;
    };

    let swapchain_format = *swapchain_format; // dereference

    const VERTICES: &[Vec2] = &[
        Vec2::new(-1., -1.),
        Vec2::new(1., -1.),
        Vec2::new(1., 1.),
        Vec2::new(-1., 1.),
    ];

    const INDICES: &[UVec3] = &[UVec3::new(0, 1, 2), UVec3::new(0, 3, 2)];

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

    let mut globals = Globals {
        color: Vec4::new(0., 0., 1., 1.),
    };

    let globals_to_buf = |globals: &Globals| {
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(globals).unwrap();
        buffer.into_inner()
    };

    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Globals UBO"),
        contents: &globals_to_buf(&globals),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    info!("Created Buffers");

    use std::mem::size_of;

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

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
                targets: &[Some(swapchain_format.into()), Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

    let window = &window;

    let mut time = 0.0f32;

    let mut previous_frame: Option<wgpu::Texture> = None;

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
                        let frame_tex_view =
                            frame.texture.create_view(&TextureViewDescriptor::default());

                        let canvas_descriptor = TextureDescriptor {
                            label: None,
                            size: frame.texture.size(),
                            mip_level_count: frame.texture.mip_level_count(),
                            sample_count: frame.texture.sample_count(),
                            dimension: frame.texture.dimension(),
                            format: frame.texture.format(),
                            usage: frame.texture.usage()
                                | TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::COPY_DST
                                | TextureUsages::COPY_SRC,
                            view_formats: &[frame.texture.format()],
                        };

                        let previous_canvas = previous_frame
                            .take()
                            .unwrap_or_else(|| device.create_texture(&canvas_descriptor));

                        let previous_canvas_view = previous_canvas.create_view(&Default::default());

                        let previous_canvas_src = device.create_texture(&TextureDescriptor {
                            usage: (frame.texture.usage()
                                | TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::COPY_DST
                                | TextureUsages::TEXTURE_BINDING),
                            ..canvas_descriptor.clone()
                        });

                        // update color in a sin-fasion
                        time += 0.01;
                        globals.color.x = time.cos().powi(2);
                        globals.color.y = time.sin().powi(2);

                        // update globals into the uniform buffer
                        queue.write_buffer(&uniform_buffer, 0, &globals_to_buf(&globals));
                        queue.submit([]);

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Triangle Enconder"),
                            });

                        encoder.copy_texture_to_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &previous_canvas,
                                mip_level: 0,
                                origin: Default::default(),
                                aspect: Default::default(),
                            },
                            wgpu::TexelCopyTextureInfo {
                                texture: &previous_canvas_src,
                                mip_level: 0,
                                origin: Default::default(),
                                aspect: Default::default(),
                            },
                            wgpu::Extent3d {
                                width: previous_canvas.width(),
                                height: previous_canvas.height(),
                                depth_or_array_layers: 0,
                            },
                        );

                        let ubo_bind_group = device.create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::Buffer(
                                        uniform_buffer.as_entire_buffer_binding(),
                                    ),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::TextureView(
                                        &previous_canvas_src
                                            .create_view(&TextureViewDescriptor::default()),
                                    ),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: wgpu::BindingResource::Sampler(&sampler),
                                },
                            ],
                        });

                        // begin render pass(es)
                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[
                                        Some(RenderPassColorAttachment {
                                            view: &frame_tex_view,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Load,
                                                // load: wgpu::LoadOp::Clear(wgpu::Color {
                                                //     r: 0.2,
                                                //     g: 0.1,
                                                //     b: 0.1,
                                                //     a: 1.,
                                                // }),
                                                store: wgpu::StoreOp::Store,
                                            },
                                            depth_slice: None,
                                        }),
                                        Some(RenderPassColorAttachment {
                                            view: &previous_canvas_view,
                                            depth_slice: None,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Load,
                                                store: wgpu::StoreOp::Store,
                                            },
                                        }),
                                    ],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    multiview_mask: None,
                                });
                            rpass.set_bind_group(0, Some(&ubo_bind_group), &[]);
                            rpass.set_pipeline(&render_pipeline);

                            rpass.set_vertex_buffer(0, vertex_buffer.slice(0..));
                            rpass.set_index_buffer(
                                index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );

                            rpass.draw_indexed(0..(3 * INDICES.len() as u32), 0, 0..1);
                        }

                        previous_frame = Some(previous_canvas);

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

// pub fn main() -> eyre::Result<()> {
//     // colored panics
//     color_eyre::install()?;
//
//     tracing_subscriber::fmt().init();
//
//     let event_loop = EventLoop::new()?;
//
//     let window = winit::window::WindowBuilder::new()
//         .with_resizable(false)
//         .with_inner_size(PhysicalSize::new(800, 800))
//         .build(&event_loop)?;
//
//     pollster::block_on(run(event_loop, window));
//
//     Ok(())
// }

fn main() -> eyre::Result<()> {
    lgpu::run()
}
