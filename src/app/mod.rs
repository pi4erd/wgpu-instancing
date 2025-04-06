mod camera;
mod draw;
mod mesh;

use std::{collections::HashMap, error::Error, sync::Arc, time::Instant};

use bytemuck::{Pod, Zeroable};
use camera::{Camera, CameraController};
use draw::Drawable;
use mesh::{DefaultVertex3d, Instance, Mesh, Vertex};
use pollster::FutureExt;
use rand::Rng;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event::WindowEvent, keyboard::{KeyCode, PhysicalKey}, window::Window};

use crate::window::Game;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InstanceRepr {
    position: [f32; 3],
}

impl InstanceRepr {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        1 => Float32x3,
    ];
}

impl Instance for InstanceRepr {
    fn attribs() -> &'static [wgpu::VertexAttribute] {
        Self::ATTRIBS
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct WorldInfo {
    time: f32,
    delta: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ComputePushConstants {
    world_info: WorldInfo,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PipelineSelector {
    Default,
    Compute,
    Custom {
        name: &'static str
    }
}

#[allow(dead_code)]
pub enum Pipeline {
    Render(wgpu::RenderPipeline),
    Compute(wgpu::ComputePipeline),
}

#[allow(dead_code)]
pub struct App<'a> {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    multisample_framebuffer: wgpu::TextureView,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    
    pipelines: HashMap<PipelineSelector, Pipeline>,
    cube_mesh: Mesh,

    camera: Camera,
    camera_controller: CameraController,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    positions: Vec<[f32; 3]>,
    velocities: Vec<[f32; 3]>,
    positions_buffer: wgpu::Buffer,
    velocities_buffer: wgpu::Buffer,
    pv_bind_group: wgpu::BindGroup,

    start_time: Instant,
    time: f64,
    last_delta: f64,
}

impl App<'_> {
    const DIMENSIONS: (u32, u32, u32) = (1024, 1024, 4);
    const WORKGROUP_DIMS: (u32, u32, u32) = (8, 8, 1);
    const OBJECT_COUNT: u32 = Self::DIMENSIONS.0 * Self::DIMENSIONS.1 * Self::DIMENSIONS.2;
    const MULTISAMPLE_SAMPLES: u32 = 8;

    fn generate_random_vectors(count: usize, min: cgmath::Point3<f32>, max: cgmath::Point3<f32>) -> Vec<cgmath::Vector3<f32>> {
        let mut vectors = Vec::with_capacity(count);
        let mut rng = rand::rng();

        for _ in 0..count {
            vectors.push(cgmath::Vector3::new(
                rng.random_range(min.x..max.x),
                rng.random_range(min.y..max.y),
                rng.random_range(min.z..max.z),
            ));
        }

        vectors
    }

    fn create_multisampled_framebuffer(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sample_count: u32
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.view_formats[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.ok_or("Failed to get adapter")?;

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("device"),
            required_features: wgpu::Features::PUSH_CONSTANTS |
                wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            required_limits: wgpu::Limits {
                max_push_constant_size: 256,
                ..Default::default()
            },
            memory_hints: wgpu::MemoryHints::Performance,
        }, None).await?;

        let size = window.inner_size();
        let mut surface_config = surface.get_default_config(&adapter, size.width, size.height)
            .expect("Surface isn't supported by adapter.");
        let view_format = surface_config.format.add_srgb_suffix();
        surface_config.view_formats.push(view_format);
        surface.configure(&device, &surface_config);

        let multisample_framebuffer = Self::create_multisampled_framebuffer(
            &device,
            &surface_config,
            Self::MULTISAMPLE_SAMPLES,
        );

        let mut pipelines = HashMap::new();

        let cube_mesh = Mesh::create(
            &device,
            &[
                DefaultVertex3d { position: [-0.5, -0.5, -0.5]},
                DefaultVertex3d { position: [0.5, -0.5, -0.5]},
                DefaultVertex3d { position: [0.5, -0.5, 0.5]},
                DefaultVertex3d { position: [-0.5, -0.5, 0.5]},

                DefaultVertex3d { position: [-0.5, 0.5, -0.5]},
                DefaultVertex3d { position: [0.5, 0.5, -0.5]},
                DefaultVertex3d { position: [0.5, 0.5, 0.5]},
                DefaultVertex3d { position: [-0.5, 0.5, 0.5]},
            ],
            &[
                // bottom
                0, 1, 2,
                0, 2, 3,

                // top
                4, 6, 5,
                4, 7, 6,

                // back
                0, 5, 1,
                0, 4, 5,

                // front
                3, 2, 6,
                3, 6, 7,

                // left
                3, 4, 0,
                3, 7, 4,

                // right
                1, 6, 2,
                1, 5, 6,
            ]
        );

        let camera = Camera::new(size.width as f32 / size.height as f32);
        let camera_controller = CameraController::new(1.0, 0.005);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera.uniform()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ]
        });

        pipelines.insert(
            PipelineSelector::Default,
            Pipeline::Render(Self::default_pipeline(
                &device,
                &[&camera_bind_group_layout],
                surface_config.format
            ))
        );

        _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
        window.set_cursor_visible(false);

        let positions = Self::generate_random_vectors(
            Self::OBJECT_COUNT as usize,
            cgmath::Point3::new(-10000.0, -10000.0, -10000.0),
            cgmath::Point3::new(10000.0, 10000.0, 10000.0),
        ).into_iter().map(|v| v.into()).collect::<Vec<_>>();
        let velocities = Self::generate_random_vectors(
            Self::OBJECT_COUNT as usize,
            cgmath::Point3::new(-1.0, -1.0, -1.0),
            cgmath::Point3::new(1.0, 1.0, 1.0),
        ).into_iter().map(|v| v.into()).collect::<Vec<_>>();

        let positions_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions_buffer"),
            contents: bytemuck::cast_slice(&positions),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        });
        let velocities_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velocities_buffer"),
            contents: bytemuck::cast_slice(&velocities),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let pv_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pv_bind_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]
        });
        let pv_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pv_bind_group"),
            layout: &pv_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocities_buffer.as_entire_binding(),
                },
            ]
        });

        pipelines.insert(PipelineSelector::Compute, Pipeline::Compute(
            Self::compute_pipeline(&device, &[&pv_bind_group_layout])
        ));

        Ok(Self {
            window,
            instance,
            surface,
            surface_config,
            multisample_framebuffer,
            adapter,
            device,
            queue,

            pipelines,
            cube_mesh,

            camera,
            camera_controller,
            camera_buffer,
            camera_bind_group,

            positions,
            velocities,
            positions_buffer,
            velocities_buffer,
            pv_bind_group,

            start_time: Instant::now(),
            time: 0.0,
            last_delta: 0.001,
        })
    }

    fn compute_pipeline(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::ComputePipeline {
        let compute_module = device.create_shader_module(wgpu::include_wgsl!("../shaders/compute.wgsl"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_pipeline_layout"),
            bind_group_layouts,
            push_constant_ranges: &[
                wgpu::PushConstantRange {
                    stages: wgpu::ShaderStages::COMPUTE,
                    range: 0..std::mem::size_of::<ComputePushConstants>() as u32,
                }
            ],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&layout),
            module: &compute_module,
            entry_point: Some("compute_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        pipeline
    }

    fn default_pipeline(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        color_format: wgpu::TextureFormat
    ) -> wgpu::RenderPipeline {
        let default_module = device.create_shader_module(wgpu::include_wgsl!("../shaders/default.wgsl"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("default_pipeline_layout"),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("default_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &default_module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    DefaultVertex3d::desc(),
                    InstanceRepr::desc(),
                ]
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            multisample: wgpu::MultisampleState {
                count: Self::MULTISAMPLE_SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &default_module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: color_format,
                        write_mask: wgpu::ColorWrites::ALL,
                        blend: None,
                    })
                ]
            }),
            depth_stencil: None,
            multiview: None,
            cache: None,
        });

        pipeline
    }

    fn update_buffers(&self) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera.uniform()]),
        );
    }
}

impl App<'_> {
    fn update(&mut self, delta: f64) {
        self.camera_controller.update(&mut self.camera, delta as f32);
        self.last_delta = delta;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_pass"),
                timestamp_writes: None,
            });

            if let Pipeline::Compute(pipeline) = &self.pipelines[&PipelineSelector::Compute] {
                compute_pass.set_pipeline(pipeline);
            }

            compute_pass.set_bind_group(0, &self.pv_bind_group, &[]);

            let push_constants = ComputePushConstants {
                world_info: WorldInfo { time: self.time as f32, delta: self.last_delta as f32 }
            };
            compute_pass.set_push_constants(0, bytemuck::bytes_of(&push_constants));

            compute_pass.dispatch_workgroups(
                Self::DIMENSIONS.0 / Self::WORKGROUP_DIMS.0,
                Self::DIMENSIONS.1 / Self::WORKGROUP_DIMS.1,
                Self::DIMENSIONS.2 / Self::WORKGROUP_DIMS.2,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let image = self.surface.get_current_texture()?;

        let view = image.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_config.view_formats[0]),
            ..Default::default()
        });

        self.update_buffers();

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisample_framebuffer,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Pipeline::Render(pipeline) = &self.pipelines[&PipelineSelector::Default] {
                render_pass.set_pipeline(pipeline);
            }

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            self.cube_mesh.draw_instanced(
                &mut render_pass,
                &self.positions_buffer,
                0..self.positions.len() as u32,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        self.window.pre_present_notify();
        image.present();

        Ok(())
    }

    fn toggle_fullscreen(&self) {
        self.window.set_fullscreen(match self.window.fullscreen() {
            None => Some(winit::window::Fullscreen::Borderless(None)),
            Some(_) => None,
        });
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.multisample_framebuffer = Self::create_multisampled_framebuffer(
            &self.device,
            &self.surface_config,
            Self::MULTISAMPLE_SAMPLES,
        );
    }
}

impl Game for App<'_> {
    fn init(window: std::sync::Arc<winit::window::Window>) -> Self {
        Self::new(window).block_on().expect("Failed to init window")
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let time = (Instant::now() - self.start_time).as_secs_f64();
        let delta = time - self.time;
        self.time = time;

        self.update(delta);
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.camera_controller.process_device_events(&event);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.camera_controller.process_window_events(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                        PhysicalKey::Code(KeyCode::KeyF) => {
                            self.toggle_fullscreen();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.window.request_redraw();

                match self.render() {
                    Ok(_) => {},
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.resize(self.window.inner_size());
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("OOM encountered. Shutting down.");
                        event_loop.exit();
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout!");
                    }
                    Err(wgpu::SurfaceError::Other) => {
                        log::warn!("Unknown surface error.")
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
            }
            _ => {}
        }
    }
}
