use std::{fs::read_to_string, iter, mem, time::Instant};

use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, ComputePipeline, Queue, RenderPipeline,
    SurfaceConfiguration, TextureFormat, TextureView,
};

use crate::{lens_state::LensState, texture::Texture};

pub struct PolyRes {
    boid_render_pipeline: wgpu::RenderPipeline,
    high_color_tex: Texture,
    conversion_render_pipeline: wgpu::RenderPipeline,
    conversion_bind_group: wgpu::BindGroup,
    compute_pipeline: ComputePipeline,
    compute_bind_group: BindGroup,
    dots_buffer: wgpu::Buffer,

    // particle_bind_groups: Vec<wgpu::BindGroup>,
    render_bind_group: wgpu::BindGroup,
    sim_param_buffer: wgpu::Buffer,
    /// ```
    /// struct SimParams {
    ///   0:opacity: f32;
    ///   1:width_scaled: f32;
    ///   2:height_scaled: f32;
    ///   3:width: f32;
    ///   4:height: f32;
    ///   5:draw_mode: f32;
    ///   6:which_ghost: f32;
    /// };
    /// ```
    pub sim_params: [f32; 7],
    pub num_dots: u32,

    convert_meta: std::fs::Metadata,
    draw_meta: std::fs::Metadata,
    compute_meta: std::fs::Metadata,
    format: TextureFormat,
    conf_format: TextureFormat,
}

impl PolyRes {
    fn shader_draw(
        device: &wgpu::Device,
        sim_params: &[f32; 7],
        sim_param_buffer: &Buffer,
        format: TextureFormat,
    ) -> (RenderPipeline, BindGroup) {
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("polyOptics"),
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_res/draw.wgsl")
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
                        // r.wavelength
                        wgpu::VertexAttribute {
                            offset: 3 * 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32,
                        },
                        // r.d
                        wgpu::VertexAttribute {
                            offset: 4 * 4,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // r.strength
                        wgpu::VertexAttribute {
                            offset: 7 * 4,
                            shader_location: 3,
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
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
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
        sim_params: &[f32; 7],
        sim_param_buffer: &Buffer,
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
                    read_to_string("gpu/src/scenes/poly_res/convert.wgsl")
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
        sim_params: &[f32; 7],
        sim_param_buffer: &Buffer,
        lens_bind_group_layout: &BindGroupLayout,
        pos_bind_group_layout: &BindGroupLayout,
        num_dots: u32,
    ) -> (ComputePipeline, BindGroup, Buffer) {
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_res/compute.wgsl")
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
                            min_binding_size: wgpu::BufferSize::new((num_dots * 32) as _),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[
                    &compute_bind_group_layout,
                    &pos_bind_group_layout,
                    lens_bind_group_layout,
                ],
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
        let initial_ray_data = vec![0.1 as f32; (num_dots * 8) as usize];

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
            ],
            label: None,
        });

        (compute_pipeline, compute_bind_group, rays_buffer)
    }

    pub async fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        lens_state: &LensState,
    ) -> Self {
        let format = wgpu::TextureFormat::Rgba16Float;
        let high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        // buffer for simulation parameters uniform
        let sim_params = [0.1, 512., 512., 512., 512., 1., 1.];
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Parameter Buffer"),
            contents: bytemuck::cast_slice(&sim_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (boid_render_pipeline, render_bind_group) =
            Self::shader_draw(device, &sim_params, &sim_param_buffer, format);

        let (conversion_render_pipeline, conversion_bind_group) = Self::convert_shader(
            device,
            &sim_params,
            &sim_param_buffer,
            &config.format,
            &high_color_tex,
        );

        let num_dots = 2;
        let (compute_pipeline, compute_bind_group, dots_buffer) = Self::raytrace_shader(
            device,
            &sim_params,
            &sim_param_buffer,
            &lens_state.lens_bind_group_layout,
            &lens_state.pos_bind_group_layout,
            num_dots,
        );

        let convert_meta = std::fs::metadata("gpu/src/scenes/poly_res/convert.wgsl").unwrap();
        let draw_meta = std::fs::metadata("gpu/src/scenes/poly_res/draw.wgsl").unwrap();
        let compute_meta = std::fs::metadata("gpu/src/scenes/poly_res/compute.wgsl").unwrap();

        Self {
            boid_render_pipeline,

            sim_param_buffer,
            sim_params,
            dots_buffer,
            render_bind_group,
            high_color_tex,
            conversion_render_pipeline,
            conversion_bind_group,
            convert_meta,
            draw_meta,
            compute_meta,
            format,
            conf_format: config.format,
            num_dots,
            compute_pipeline,
            compute_bind_group,
        }
    }

    pub fn write_buffer(&self, queue: &Queue) {
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );
    }

    pub fn update_dots(
        &mut self,
        device: &wgpu::Device,
        queue: &Queue,
        update_size: bool,
        lens_state: &LensState,
    ) {
        if update_size {
            println!("update: {}", self.num_dots);
            let (compute_pipeline, compute_bind_group, dots_buffer) = Self::raytrace_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &lens_state.lens_bind_group_layout,
                &lens_state.pos_bind_group_layout,
                self.num_dots,
            );
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.dots_buffer = dots_buffer;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let work_group_count = (self.num_dots + 64 - 1) / 64; // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.pos_bind_group, &[]);
            cpass.set_bind_group(2, &lens_state.lens_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }

        if cfg!(debug_assertions) & false {
            let output_buffer_size = (self.num_dots * 32) as wgpu::BufferAddress;
            let output_buffer_desc = wgpu::BufferDescriptor {
                size: output_buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                label: Some("Ray DST"),
                mapped_at_creation: false,
            };
            let output_buffer = device.create_buffer(&output_buffer_desc);
            encoder.copy_buffer_to_buffer(
                &self.dots_buffer,
                0,
                &output_buffer,
                0,
                (self.num_dots * 32).into(),
            );

            queue.submit(iter::once(encoder.finish()));

            let buffer_slice = output_buffer.slice(..);
            let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
            device.poll(wgpu::Maintain::Wait);

            if let Ok(()) = pollster::block_on(buffer_future) {
                let data = buffer_slice.get_mapped_range();

                let vertices = unsafe { data.align_to::<f32>().1 };
                let vec_vertices = vertices.to_vec();
                let data = vec_vertices;

                println!("----------------------------------------------------------------------------------");
                for elements in data.chunks(8) {
                    print!("o: {}, {}, {}  ", elements[0], elements[1], elements[2]);
                    print!("d: {}, {}, {}  ", elements[4], elements[5], elements[6]);
                    println!("s: {}", elements[7]);
                }
                // println!("{:?}", data);
            } else {
                panic!("Failed to copy ray buffer!")
            }
        } else {
            queue.submit(iter::once(encoder.finish()));
        }
    }
}

impl PolyRes {
    pub fn resize(
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
        let mut config = config.clone();
        let scale_fact = 1.;
        config.width = (new_size.width as f64 * scale_factor * scale_fact) as u32;
        config.height = (new_size.height as f64 * scale_factor * scale_fact) as u32;

        self.high_color_tex =
            Texture::create_color_texture(device, &config, format, "high_color_tex");

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
            ],
            label: Some("texture_bind_group"),
        });

        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );
    }

    pub fn update(&mut self, device: &wgpu::Device, lens_state: &LensState) {
        // if self.cell_timer.elapsed().unwrap().as_secs_f32() > 0.1 {
        //     self.cell_timer = SystemTime::now();
        //     self.update_cells(device, queue);
        // }
        //self.update_rays(device);

        if self.convert_meta.modified().unwrap()
            != std::fs::metadata("gpu/src/scenes/poly_res/convert.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            print!("reloading convert shader! ");
            let now = Instant::now();
            self.convert_meta = std::fs::metadata("gpu/src/scenes/poly_res/convert.wgsl").unwrap();
            let (pipeline, bind_group) = Self::convert_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &self.conf_format,
                &self.high_color_tex,
            );
            self.conversion_render_pipeline = pipeline;
            self.conversion_bind_group = bind_group;
            println!("took {:?}.", now.elapsed());
        }

        if self.draw_meta.modified().unwrap()
            != std::fs::metadata("gpu/src/scenes/poly_res/draw.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            self.draw_meta = std::fs::metadata("gpu/src/scenes/poly_res/draw.wgsl").unwrap();
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
            != std::fs::metadata("gpu/src/scenes/poly_res/compute.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            self.compute_meta = std::fs::metadata("gpu/src/scenes/poly_res/compute.wgsl").unwrap();
            print!("reloading compute shader! ");
            let now = Instant::now();
            let (pipeline, bind_group, dots_buffer) = Self::raytrace_shader(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                &lens_state.lens_bind_group_layout,
                &lens_state.pos_bind_group_layout,
                self.num_dots,
            );
            self.compute_pipeline = pipeline;
            self.compute_bind_group = bind_group;
            self.dots_buffer = dots_buffer;
            println!("took {:?}.", now.elapsed());
        }
    }

    pub fn render(
        &mut self,
        view: &TextureView,
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
            rpass.set_vertex_buffer(0, self.dots_buffer.slice(..));
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw(0..self.num_dots, 0..1);
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

            // println!("{},{},{}", rays[3], rays[4], rays[5]);

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

    pub fn render_hires(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        num_rays: u64,
        lens_state: &mut LensState,
    ) -> Result<(), wgpu::SurfaceError> {
        let num_per_iter = 10_000_000;

        let iters = ((num_rays / num_per_iter) as f32).sqrt() as u32;
        let width = lens_state.pos_params[9] / iters as f32;
        println!("iters: {}, width: {}", iters, width);
        let old_width = lens_state.pos_params[9];
        lens_state.pos_params[9] = width;
        let num_dots = self.num_dots;
        self.num_dots = num_per_iter as u32;

        println!("opacity_mul: {}", (num_dots as f64 / num_rays as f64) as f32);
        let opacity = self.sim_params[0];
        self.sim_params[0] *= 2. * (num_dots as f64 / num_rays as f64) as f32;
        self.write_buffer(queue);

        lens_state.update(device, queue);

        let old_x = lens_state.pos_params[4];
        let old_y = lens_state.pos_params[5];

        let mut first_pass = true;
        for i in 0..iters {
            for j in 0..iters {
                lens_state.pos_params[4] = (i as f32 + 0.5) * width - (width * iters as f32 / 2.); //x center
                lens_state.pos_params[5] = (j as f32 + 0.5) * width - (width * iters as f32 / 2.); //y center
                                                                                                   // println!("center: {},{}", lens_state.pos_params[4], lens_state.pos_params[5]);
                lens_state.update(device, queue);
                self.update_dots(device, queue, first_pass, lens_state);
                // pollster::block_on(queue.on_submitted_work_done());
                std::thread::sleep(core::time::Duration::from_millis(20));

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                // create render pass descriptor and its color attachments
                let color_attachments = [wgpu::RenderPassColorAttachment {
                    view: &self.high_color_tex.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // clear if on first render
                        load: if first_pass {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            })
                        } else {
                            wgpu::LoadOp::Load
                        },
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
                    rpass.set_vertex_buffer(0, self.dots_buffer.slice(..));
                    //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                    rpass.draw(0..self.num_dots, 0..1);
                }

                queue.submit(iter::once(encoder.finish()));
                // std::thread::sleep(core::time::Duration::from_millis(100));
                // pollster::block_on(queue.on_submitted_work_done());
                first_pass = false;
            }
        }

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

            // println!("{},{},{}", rays[3], rays[4], rays[5]);

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

        self.num_dots = num_dots;
        lens_state.pos_params[9] = old_width;

        lens_state.pos_params[4] = old_x;
        lens_state.pos_params[5] = old_y;

        self.sim_params[0] = opacity;
        lens_state.update(device, queue);
        self.update_dots(device, queue, true, lens_state);
        Ok(())
    }
}
