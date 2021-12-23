use std::{fs::read_to_string, iter, time::Instant};

use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, ComputePipeline, Queue, RenderPipeline,
    SurfaceConfiguration, TextureFormat, TextureView,
};

use crate::{lens_state::LensState, texture::Texture};

/// 2d View of the lens and rays passing through it
pub struct PolyOptics {
    /// Pipeline for ray rendering
    render_pipeline: wgpu::RenderPipeline,
    /// Texture to render into initally
    /// - needs higher color resolution for smooth blending
    high_color_tex: Texture,
    /// Pipeline for conversion of color spaces
    conversion_render_pipeline: wgpu::RenderPipeline,
    /// Bind group for conversion of color spaces
    conversion_bind_group: wgpu::BindGroup,
    /// Pipeline for generating rays
    compute_pipeline: ComputePipeline,
    compute_bind_group: BindGroup,
    /// Buffer to hold generated rays
    rays_buffer: wgpu::Buffer,

    pub num_rays: u32,

    /// Metadata to keep track of shader
    convert_meta: std::fs::Metadata,
    /// Metadata to keep track of shader
    draw_meta: std::fs::Metadata,
    /// Metadata to keep track of shader
    compute_meta: std::fs::Metadata,

    /// format of the high color Texture
    format: TextureFormat,
    /// format of the final Texture
    conf_format: TextureFormat,
}

impl PolyOptics {
    /// (re)load ray drawing shader.
    /// It will read the shader code from disk
    fn shader_draw(
        device: &wgpu::Device,
        format: TextureFormat,
        params_bind_group_layout: &BindGroupLayout,
    ) -> RenderPipeline {
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("polyOptics"),
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_optics/draw.wgsl")
                    .expect("Shader could not be read.")
                    .into(),
            ),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[&params_bind_group_layout],
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

        boid_render_pipeline
    }

    /// (re)load ray color space conversion shader.
    /// It will read the shader code from disk
    ///
    /// This shader will also draw the reprensation lens
    fn convert_shader(
        device: &wgpu::Device,
        lens_bind_group_layout: &BindGroupLayout,
        params_bind_group_layout: &BindGroupLayout,
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
            ],
            label: Some("texture_bind_group"),
        });

        let conversion_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Conversion Pipeline Layout"),
                bind_group_layouts: &[
                    &conversion_bind_group_layout,
                    &params_bind_group_layout,
                    lens_bind_group_layout,
                ],
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

    /// (re)load ray tracing shader.
    /// It will read the shader code from disk
    fn raytrace_shader(
        device: &wgpu::Device,
        lens_bind_group_layout: &BindGroupLayout,
        params_bind_group_layout: &BindGroupLayout,
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
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[
                    &compute_bind_group_layout,
                    &params_bind_group_layout,
                    &lens_bind_group_layout,
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
        let initial_ray_data = vec![0 as f32; (num_rays * 8) as usize];

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
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: rays_buffer.as_entire_binding(),
            }],
            label: None,
        });

        (compute_pipeline, compute_bind_group, rays_buffer)
    }

    pub async fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        lens_state: &LensState,
    ) -> Self {
        // let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        //     label: None,
        //     source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        // });

        let format = wgpu::TextureFormat::Rgba16Float;
        let high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        let boid_render_pipeline =
            Self::shader_draw(device, format, &lens_state.params_bind_group_layout);

        let (conversion_render_pipeline, conversion_bind_group) = Self::convert_shader(
            device,
            &lens_state.lens_bind_group_layout,
            &lens_state.params_bind_group_layout,
            &config.format,
            &high_color_tex,
        );

        let num_rays = 2;
        let (compute_pipeline, compute_bind_group, rays_buffer) = Self::raytrace_shader(
            device,
            &lens_state.lens_bind_group_layout,
            &lens_state.params_bind_group_layout,
            num_rays,
        );

        let convert_meta = std::fs::metadata("gpu/src/scenes/poly_optics/convert.wgsl").unwrap();
        let draw_meta = std::fs::metadata("gpu/src/scenes/poly_optics/draw.wgsl").unwrap();
        let compute_meta = std::fs::metadata("gpu/src/scenes/poly_optics/compute.wgsl").unwrap();

        Self {
            render_pipeline: boid_render_pipeline,

            num_rays,
            high_color_tex,
            conversion_render_pipeline,
            conversion_bind_group,
            compute_pipeline,
            compute_bind_group,
            rays_buffer,
            convert_meta,
            draw_meta,
            compute_meta,
            format,
            conf_format: config.format,
        }
    }

    pub fn update_rays(
        &mut self,
        device: &wgpu::Device,
        queue: &Queue,
        update_size: bool,
        lens_state: &LensState,
    ) {
        if update_size {
            // println!("update: {}", self.num_rays);
            let (compute_pipeline, compute_bind_group, rays_buffer) = Self::raytrace_shader(
                device,
                &lens_state.lens_bind_group_layout,
                &lens_state.params_bind_group_layout,
                self.num_rays,
            );
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.rays_buffer = rays_buffer;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let work_group_count = (self.num_rays + 64 - 1) / 64; // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            cpass.set_bind_group(2, &lens_state.lens_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }

        if cfg!(debug_assertions) & false {
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

    pub fn update_buffers(
        &mut self,
        device: &wgpu::Device,
        update_size: bool,
        lens_state: &LensState,
    ) {
        if update_size {
            // buffer for elements
            let (pipeline, bind_group) = Self::convert_shader(
                device,
                &lens_state.lens_bind_group_layout,
                &lens_state.params_bind_group_layout,
                &self.conf_format,
                &self.high_color_tex,
            );
            self.conversion_render_pipeline = pipeline;
            self.conversion_bind_group = bind_group;
        }

        let (compute_pipeline, compute_bind_group, rays_buffer) = Self::raytrace_shader(
            device,
            &lens_state.lens_bind_group_layout,
            &lens_state.params_bind_group_layout,
            self.num_rays,
        );
        self.compute_pipeline = compute_pipeline;
        self.compute_bind_group = compute_bind_group;
        self.rays_buffer = rays_buffer;

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
            ],
            label: Some("texture_bind_group"),
        });
    }
}

impl PolyOptics {
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        lens_state: &LensState,
    ) {
        let format = wgpu::TextureFormat::Rgba16Float;
        self.high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        self.update_buffers(device, false, lens_state);
    }

    pub fn update(&mut self, device: &wgpu::Device, lens_state: &LensState) {
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
            let (pipeline, bind_group) = Self::convert_shader(
                device,
                &lens_state.lens_bind_group_layout,
                &lens_state.params_bind_group_layout,
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
            let pipeline =
                Self::shader_draw(device, self.format, &lens_state.params_bind_group_layout);
            self.render_pipeline = pipeline;
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
            let (compute_pipeline, compute_bind_group, rays_buffer) = Self::raytrace_shader(
                device,
                &lens_state.lens_bind_group_layout,
                &lens_state.params_bind_group_layout,
                self.num_rays,
            );
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.rays_buffer = rays_buffer;
            println!("took {:?}.", now.elapsed());
        }
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        lens_state: &LensState,
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
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &lens_state.params_bind_group, &[]);
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
                label: Some("Poly_optics conversion Encoder"),
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
                label: Some("poly_optics conversion"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
            };

            //let rays = vec![-1.0, -1.0, 0.0, 0.0, 1.0, 1.0];
            {
                // render pass
                let mut rpass = encoder.begin_render_pass(&render_pass_descriptor);
                rpass.set_pipeline(&self.conversion_render_pipeline);
                rpass.set_bind_group(0, &self.conversion_bind_group, &[]);
                rpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
                rpass.set_bind_group(2, &lens_state.lens_bind_group, &[]);
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
