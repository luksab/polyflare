use std::{fs::read_to_string, iter, mem, time::Instant};

use cgmath::{InnerSpace, Vector3};
use wgpu::{
    util::DeviceExt, BindGroup, Buffer, ComputePipeline, Queue, RenderPipeline,
    SurfaceConfiguration, TextureFormat, TextureView,
};

use crate::{scene::Scene, texture::Texture};

use polynomial_optics::*;

pub struct PolyOptics {
    boid_render_pipeline: wgpu::RenderPipeline,
    high_color_tex: Texture,
    conversion_render_pipeline: wgpu::RenderPipeline,
    conversion_bind_group: wgpu::BindGroup,
    compute_pipeline: ComputePipeline,
    compute_bind_group: BindGroup,
    rays_buffer: wgpu::Buffer,
    lens_buffer: wgpu::Buffer,
    lens_data: Vec<f32>,

    // particle_bind_groups: Vec<wgpu::BindGroup>,
    render_bind_group: wgpu::BindGroup,
    sim_param_buffer: wgpu::Buffer,
    pub sim_params: [f32; 5],
    // compute_pipeline: wgpu::ComputePipeline,
    // work_group_count: u32,
    // frame_num: usize,
    // cell_timer: SystemTime,
    pub lens: Lens,
    pub rays: Vec<f32>,
    pub num_rays: u32,
    pub which_ghost: u32,
    pub draw_mode: u32,
    pub center_pos: Vector3<f64>,
    pub direction: Vector3<f64>,

    convert_meta: std::fs::Metadata,
    draw_meta: std::fs::Metadata,
    compute_meta: std::fs::Metadata,
    format: TextureFormat,
    conf_format: TextureFormat,
}

impl PolyOptics {
    fn shader_draw(
        device: &wgpu::Device,
        sim_params: &[f32; 5],
        sim_param_buffer: &Buffer,
        format: TextureFormat,
    ) -> (RenderPipeline, BindGroup) {
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("polyOptics"),
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_optics/draw.wgsl")
                    .expect("Shader could not be read.")
                    .into(),
            ),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (sim_params.len() * mem::size_of::<f32>()) as _,
                        ),
                    },
                    count: None,
                }],
                label: None,
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let boid_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // r.o
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // r.d
                        wgpu::VertexAttribute {
                            offset: 4 * 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // r.strength
                        wgpu::VertexAttribute {
                            offset: 7 * 4,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                clamp_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sim_param_buffer.as_entire_binding(),
            }],
            label: None,
        });

        (boid_render_pipeline, render_bind_group)
    }

    fn convert_shader(
        device: &wgpu::Device,
        sim_params: &[f32; 5],
        sim_param_buffer: &Buffer,
        lens_data: &Vec<f32>,
        lens_buffer: &Buffer,
        format: &TextureFormat,
        high_color_tex: &Texture,
    ) -> (RenderPipeline, BindGroup) {
        let conversion_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_params.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (lens_data.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let conversion_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &conversion_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&high_color_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&high_color_tex.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: lens_buffer.as_entire_binding(),
                },
            ],
            label: Some("texture_bind_group"),
        });
        let conversion_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Conversion Pipeline Layout"),
                bind_group_layouts: &[&conversion_bind_group_layout],
                push_constant_ranges: &[],
            });
        let conversion_render_pipeline = {
            let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("conversion"),
                source: wgpu::ShaderSource::Wgsl(
                    read_to_string("gpu/src/scenes/poly_optics/convert.wgsl")
                        .expect("Shader could not be read.")
                        .into(),
                ),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&conversion_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 2 * 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "main",
                    targets: &[wgpu::ColorTargetState {
                        format: *format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLAMPING
                    clamp_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
            })
        };
        (conversion_render_pipeline, conversion_bind_group)
    }

    fn raytrace_shader(
        device: &wgpu::Device,
        sim_params: &[f32; 5],
        sim_param_buffer: &Buffer,
        lens_data: &Vec<f32>,
        lens_buffer: &Buffer,
        num_rays: u32,
    ) -> (ComputePipeline, BindGroup, Buffer) {
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_optics/compute.wgsl")
                    .expect("Shader could not be read.")
                    .into(),
            ),
        });

        // create compute bind layout group and compute pipeline layout
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_params.len() * mem::size_of::<u32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((num_rays * 32) as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (lens_data.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        // create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // buffer for all particles data of type [bool,...]
        // vec3: 16 bytes, 4 floats
        // vec3, vec3, float
        let initial_ray_data = vec![0 as f32; (num_rays * 8) as usize];
        println!("new ray buffer [{}]", num_rays * 8);
        // for (i, particle_instance_chunk) in &mut initial_particle_data.chunks_mut(2).enumerate() {
        //     particle_instance_chunk[0] = i as u32; // bool??
        //     particle_instance_chunk[1] = fastrand::f32(0..6) / 5; // bool??
        // }

        let rays_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Rays Buffer")),
            contents: bytemuck::cast_slice(&initial_ray_data),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC,
        });

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lens_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        (compute_pipeline, compute_bind_group, rays_buffer)
    }

    pub async fn new(device: &wgpu::Device, config: &SurfaceConfiguration) -> Self {
        // let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        //     label: None,
        //     source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        // });

        let format = wgpu::TextureFormat::Rgba16Float;
        let high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        // buffer for simulation parameters uniform
        let sim_params = [0.1, 512., 512., 512., 512.];
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Parameter Buffer"),
            contents: bytemuck::cast_slice(&sim_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (boid_render_pipeline, render_bind_group) =
            Self::shader_draw(device, &sim_params, &sim_param_buffer, format);

        let lens = {
            let radius = 3.0;
            let lens_entry = Element {
                radius,
                glass: Glass {
                    ior: 1.5,
                    coating: (),
                },
                position: -2.0,
                entry: true,
                spherical: true,
            };
            let lens_exit_pos = 1.0;
            let lens_exit = Element {
                radius,
                glass: Glass {
                    ior: 1.5,
                    coating: (),
                },
                position: lens_exit_pos,
                entry: false,
                spherical: true,
            };

            Lens::new(vec![lens_entry, lens_exit])
        };

        // buffer for elements
        let lens_data = lens.get_elements_buffer();
        let lens_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lens drawing Buffer"),
            contents: bytemuck::cast_slice(&lens_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let (conversion_render_pipeline, conversion_bind_group) = Self::convert_shader(
            device,
            &sim_params,
            &sim_param_buffer,
            &lens_data,
            &lens_buffer,
            &config.format,
            &high_color_tex,
        );

        let num_rays = 2;
        let (compute_pipeline, compute_bind_group, rays_buffer) = Self::raytrace_shader(
            device,
            &sim_params,
            &sim_param_buffer,
            &lens_data,
            &lens_buffer,
            num_rays,
        );

        let convert_meta = std::fs::metadata("gpu/src/scenes/poly_optics/convert.wgsl").unwrap();
        let draw_meta = std::fs::metadata("gpu/src/scenes/poly_optics/draw.wgsl").unwrap();
        let compute_meta = std::fs::metadata("gpu/src/scenes/poly_optics/compute.wgsl").unwrap();

        Self {
            boid_render_pipeline,

            sim_param_buffer,
            sim_params,
            render_bind_group,
            lens,

            draw_mode: 2,
            num_rays,
            which_ghost: 2,
            rays: vec![],
            center_pos: Vector3 {
                x: 0.0,
                y: 0.0,
                z: -5.,
            },
            direction: Vector3 {
                x: 0.0,
                y: 0.1,
                z: 1.0,
            }
            .normalize(),
            high_color_tex,
            conversion_render_pipeline,
            conversion_bind_group,
            compute_pipeline,
            compute_bind_group,
            rays_buffer,
            lens_data,
            lens_buffer,
            convert_meta,
            draw_meta,
            compute_meta,
            format,
            conf_format: config.format,
        }
    }

    pub fn write_buffer(&self, queue: &Queue) {
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );
    }

    pub async fn update_rays(&mut self, device: &wgpu::Device, queue: &Queue, update_size: bool) {
        if update_size {
            println!("update: {}", self.num_rays);
            let (compute_pipeline, compute_bind_group, rays_buffer) = Self::raytrace_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &self.lens_data,
                &self.lens_buffer,
                self.num_rays,
            );
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.rays_buffer = rays_buffer;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let work_group_count = 1;
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }

        if cfg!(debug_assertions) {
            let output_buffer_size = (self.num_rays * 32) as wgpu::BufferAddress;
            let output_buffer_desc = wgpu::BufferDescriptor {
                size: output_buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                label: Some("Ray DST"),
                mapped_at_creation: false,
            };
            let output_buffer = device.create_buffer(&output_buffer_desc);
            encoder.copy_buffer_to_buffer(
                &self.rays_buffer,
                0,
                &output_buffer,
                0,
                (self.num_rays * 32).into(),
            );

            queue.submit(iter::once(encoder.finish()));

            let buffer_slice = output_buffer.slice(..);
            let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
            device.poll(wgpu::Maintain::Wait);

            if let Ok(()) = buffer_future.await {
                let data = buffer_slice.get_mapped_range();

                let vertices = unsafe { data.align_to::<f32>().1 };
                let vec_vertices = vertices.to_vec();
                let data = vec_vertices;
                println!("{:?}", data);
            } else {
                panic!("Failed to copy ray buffer!")
            }
        }

        // self.rays = self.lens.get_rays(
        //     self.num_rays,
        //     self.center_pos,
        //     self.direction.normalize(),
        //     self.draw_mode,
        //     self.which_ghost,
        // );
        // self.vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Vertex Buffer"),
        //     contents: bytemuck::cast_slice(&self.rays[..]),
        //     usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        // });

        // let rays = self.lens.get_rays(self.num_rays, self.center_pos);

        // let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //     label: Some("Render Encoder"),
        // });
        // // {
        // //     let mut cpass =
        // //         encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        // //     cpass.set_pipeline(&self.compute_pipeline);
        // //     cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
        // //     cpass.dispatch(self.work_group_count, 1, 1);
        // // }
        // queue.submit(iter::once(encoder.finish()));
    }

    pub fn update_buffers(&mut self, queue: &Queue, device: &wgpu::Device, update_size: bool) {
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );

        if update_size {
            // buffer for elements
            let lens_data = self.lens.get_elements_buffer();
            let lens_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Lens drawing Buffer"),
                contents: bytemuck::cast_slice(&lens_data),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::STORAGE,
            });
            let (pipeline, bind_group) = Self::convert_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &lens_data,
                &lens_buffer,
                &self.conf_format,
                &self.high_color_tex,
            );
            self.conversion_render_pipeline = pipeline;
            self.conversion_bind_group = bind_group;
        }

        // buffer for elements
        let lens_data = self.lens.get_elements_buffer();
        let lens_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lens drawing Buffer"),
            contents: bytemuck::cast_slice(&lens_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let conversion_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (self.sim_params.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (lens_data.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        self.conversion_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &conversion_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.high_color_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.high_color_tex.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.sim_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: lens_buffer.as_entire_binding(),
                },
            ],
            label: Some("texture_bind_group"),
        });
    }
}

impl Scene for PolyOptics {
    fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        queue: &Queue,
    ) {
        self.sim_params[1] = new_size.width as f32 * scale_factor as f32;
        self.sim_params[2] = new_size.height as f32 * scale_factor as f32;
        self.sim_params[3] = new_size.width as f32;
        self.sim_params[4] = new_size.height as f32;

        let format = wgpu::TextureFormat::Rgba16Float;
        self.high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        self.update_buffers(queue, device, false);
    }

    fn input(&mut self, _event: &winit::event::WindowEvent) -> bool {
        true
    }

    fn update(&mut self, _dt: std::time::Duration, device: &wgpu::Device, _queue: &Queue) {
        // if self.cell_timer.elapsed().unwrap().as_secs_f32() > 0.1 {
        //     self.cell_timer = SystemTime::now();
        //     self.update_cells(device, queue);
        // }
        //self.update_rays(device);

        if self.convert_meta.modified().unwrap()
            != std::fs::metadata("gpu/src/scenes/poly_optics/convert.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            print!("reloading convert shader! ");
            let now = Instant::now();
            self.convert_meta =
                std::fs::metadata("gpu/src/scenes/poly_optics/convert.wgsl").unwrap();
            // buffer for elements
            let lens_data = self.lens.get_elements_buffer();
            let lens_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Lens drawing Buffer"),
                contents: bytemuck::cast_slice(&lens_data),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::STORAGE,
            });
            let (pipeline, bind_group) = Self::convert_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &lens_data,
                &lens_buffer,
                &self.conf_format,
                &self.high_color_tex,
            );
            self.conversion_render_pipeline = pipeline;
            self.conversion_bind_group = bind_group;
            println!("took {:?}.", now.elapsed());
        }

        if self.draw_meta.modified().unwrap()
            != std::fs::metadata("gpu/src/scenes/poly_optics/draw.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            self.draw_meta = std::fs::metadata("gpu/src/scenes/poly_optics/draw.wgsl").unwrap();
            print!("reloading draw shader! ");
            let now = Instant::now();
            let (pipeline, bind_group) = Self::shader_draw(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                self.format,
            );
            self.boid_render_pipeline = pipeline;
            self.render_bind_group = bind_group;
            println!("took {:?}.", now.elapsed());
        }

        if self.compute_meta.modified().unwrap()
            != std::fs::metadata("gpu/src/scenes/poly_optics/compute.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            self.compute_meta =
                std::fs::metadata("gpu/src/scenes/poly_optics/compute.wgsl").unwrap();
            print!("reloading compute shader! ");
            let now = Instant::now();
            let (pipeline, bind_group, rays_buffer) = Self::raytrace_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &self.lens_data,
                &self.lens_buffer,
                self.num_rays,
            );
            self.compute_pipeline = pipeline;
            self.compute_bind_group = bind_group;
            self.rays_buffer = rays_buffer;
            println!("took {:?}.", now.elapsed());
        }
    }

    fn render(
        &mut self,
        view: &TextureView,
        _depth_view: Option<&TextureView>,
        device: &wgpu::Device,
        queue: &Queue,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // create render pass descriptor and its color attachments
        let color_attachments = [wgpu::RenderPassColorAttachment {
            view: &self.high_color_tex.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: true,
            },
        }];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
        };

        // println!("{},{},{}", rays[3], rays[4], rays[5]);

        //let rays = vec![-1.0, -1.0, 0.0, 0.0, 1.0, 1.0];
        {
            // render pass
            let mut rpass = encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.boid_render_pipeline);
            rpass.set_bind_group(0, &self.render_bind_group, &[]);
            // the three instance-local vertices
            rpass.set_vertex_buffer(0, self.rays_buffer.slice(..));
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw(0..self.num_rays, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));

        // conversion pass
        {
            // let vertex_buffer_data = [
            //     -0.1f32, -0.1, 0.1, -0.1, -0.1, 0.1, -0.1, 0.1, 0.1, 0.1, 0.1, -0.1,
            // ];
            let vertex_buffer_data = [
                -1.0f32, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0,
            ];
            let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::bytes_of(&vertex_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            // create render pass descriptor and its color attachments
            let color_attachments = [wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: true,
                },
            }];
            let render_pass_descriptor = wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
            };

            //let rays = vec![-1.0, -1.0, 0.0, 0.0, 1.0, 1.0];
            {
                // render pass
                let mut rpass = encoder.begin_render_pass(&render_pass_descriptor);
                rpass.set_pipeline(&self.conversion_render_pipeline);
                rpass.set_bind_group(0, &self.conversion_bind_group, &[]);
                // the three instance-local vertices
                rpass.set_vertex_buffer(0, vertices_buffer.slice(..));
                //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                rpass.draw(0..vertex_buffer_data.len() as u32 / 2, 0..1);
            }

            queue.submit(iter::once(encoder.finish()));
        }

        Ok(())
    }
}
