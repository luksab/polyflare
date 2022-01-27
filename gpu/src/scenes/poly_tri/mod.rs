use std::{fs::read_to_string, iter, time::Instant};

use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, CommandEncoder, ComputePipeline, Queue,
    RenderPipeline, SurfaceConfiguration, TextureFormat, TextureView,
};

use crate::{lens_state::LensState, texture::Texture};

pub struct PolyTri {
    tri_render_pipeline: wgpu::RenderPipeline,
    triangulate_pipeline: wgpu::ComputePipeline,
    high_color_tex: Texture,
    conversion_render_pipeline: wgpu::RenderPipeline,
    conversion_bind_group: wgpu::BindGroup,
    compute_pipeline: ComputePipeline,
    compute_bind_group: BindGroup,
    compute_bind_group_layout: BindGroupLayout,
    pub vertex_buffer: wgpu::Buffer,
    tri_index_buffer: wgpu::Buffer,

    pub dot_side_len: u32,

    convert_meta: Option<std::fs::Metadata>,
    draw_meta: Option<std::fs::Metadata>,
    compute_meta: Option<std::fs::Metadata>,
    triangulate_meta: Option<std::fs::Metadata>,
    format: TextureFormat,
    conf_format: TextureFormat,
}

impl PolyTri {
    fn shader_draw(
        device: &wgpu::Device,
        format: TextureFormat,
        params_bind_group_layout: &BindGroupLayout,
        lens_bind_group_layout: &BindGroupLayout,
    ) -> RenderPipeline {
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("polyOptics"),
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_tri/draw.wgsl")
                    .unwrap_or_else(|_| include_str!("draw.wgsl").to_string())
                    .into(),
            ),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[lens_bind_group_layout, params_bind_group_layout],
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "mainv",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // aperture position
                        wgpu::VertexAttribute {
                            offset: 2 * 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // entry aperture position
                        wgpu::VertexAttribute {
                            offset: 4 * 4,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // strength
                        wgpu::VertexAttribute {
                            offset: 6 * 4,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32,
                        },
                        // wavelength
                        wgpu::VertexAttribute {
                            offset: 7 * 4,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "mainf",
                targets: &[wgpu::ColorTargetState {
                    format,
                    // blend: None,
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                // topology: wgpu::PrimitiveTopology::PointList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    fn convert_shader(
        device: &wgpu::Device,
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                bind_group_layouts: &[&conversion_bind_group_layout, params_bind_group_layout],
                push_constant_ranges: &[],
            });
        let conversion_render_pipeline = {
            let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("conversion"),
                source: wgpu::ShaderSource::Wgsl(
                    read_to_string("gpu/src/scenes/poly_tri/convert.wgsl")
                        .unwrap_or_else(|_| include_str!("convert.wgsl").to_string())
                        .into(),
                ),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&conversion_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "mainv",
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
                    entry_point: "mainf",
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
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                    unclipped_depth: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        (conversion_render_pipeline, conversion_bind_group)
    }

    fn raytrace_shader(
        device: &wgpu::Device,
        lens_bind_group_layout: &BindGroupLayout,
        params_bind_group_layout: &BindGroupLayout,
        dot_side_len: u32,
        num_ghosts: u32,
    ) -> (ComputePipeline, BindGroup, BindGroupLayout, Buffer) {
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_tri/compute.wgsl")
                    .unwrap_or_else(|_| include_str!("compute.wgsl").to_string())
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
                    lens_bind_group_layout,
                    params_bind_group_layout,
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
        let initial_ray_data =
            vec![0.1_f32; (dot_side_len * dot_side_len * num_ghosts * 8) as usize];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&"Rays Buffer".to_string()),
            contents: bytemuck::cast_slice(&initial_ray_data),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            }],
            label: None,
        });

        (
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            vertex_buffer,
        )
    }

    fn triangulate_shader(
        device: &wgpu::Device,
        compute_bind_group_layout: &BindGroupLayout,
        params_bind_group_layout: &BindGroupLayout,
        dot_side_len: u32,
        num_ghosts: u32,
    ) -> (ComputePipeline, Buffer) {
        let triangulate_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/scenes/poly_tri/triangulate.wgsl")
                    .unwrap_or_else(|_| include_str!("triangulate.wgsl").to_string())
                    .into(),
            ),
        });

        let triangulate_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("triangulate"),
                bind_group_layouts: &[compute_bind_group_layout, params_bind_group_layout],
                push_constant_ranges: &[],
            });

        // create compute pipeline
        let triangulate_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Triangulate pipeline"),
                layout: Some(&triangulate_pipeline_layout),
                module: &triangulate_shader,
                entry_point: "main",
            });

        (
            triangulate_pipeline,
            Self::get_tri_index(device, dot_side_len, num_ghosts),
        )
    }

    fn get_tri_index(device: &wgpu::Device, dot_side_len: u32, num_ghosts: u32) -> Buffer {
        // buffer for all verts data of type
        // vec2: 8 bytes, 2 floats
        // pos: vec2, aperture_pos: vec2, intensity: float + 1float alignment
        // let num_tris = (((dot_side_len - 1) * 2) * (dot_side_len - 1)) as usize;
        let mut initial_tri_index_data = vec![]; //0 as u32; (num_tris * 6) as usize];

        for i in 0..num_ghosts {
            let offset = i * (dot_side_len * dot_side_len);
            for y in 0..dot_side_len - 1 {
                for x in 0..dot_side_len - 1 {
                    // println!("x:{} y:{}", x, y);
                    initial_tri_index_data.push(x + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + 1 + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + dot_side_len + y * dot_side_len + offset);

                    initial_tri_index_data.push(x + 1 + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + dot_side_len + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + 1 + dot_side_len + y * dot_side_len + offset);
                }
            }
        }

        if cfg!(debug_assertions) {
            println!(
                "initial_tri_index_data: {} {:?}",
                dot_side_len, initial_tri_index_data
            );
        }

        // assert_eq!(initial_tri_index_data.len() / 3, num_tris);

        // let now = Instant::now();
        // for i in 0..num_ghosts {
        //     initial_tri_index_data
        //         .append(&mut initial_tri_index_data.iter().map(|x| *x + offset).collect());
        // }
        // if cfg!(debug_assertions) {
        //     println!("Triangulate index buffer: {:?}", initial_tri_index_data);
        // }
        // println!("creating {} took {:?}", num_ghosts, now.elapsed());

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&"Tris Buffer".to_string()),
            contents: bytemuck::cast_slice(&initial_tri_index_data),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn get_num_tris(&self) -> u32 {
        (self.dot_side_len - 1) * (self.dot_side_len - 1)
    }

    pub async fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        lens_state: &LensState,
    ) -> Self {
        let format = wgpu::TextureFormat::Rgba16Float;
        let high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        let tri_render_pipeline = Self::shader_draw(
            device,
            format,
            &lens_state.params_bind_group_layout,
            &lens_state.lens_bind_group_layout,
        );

        let (conversion_render_pipeline, conversion_bind_group) = Self::convert_shader(
            device,
            &lens_state.params_bind_group_layout,
            &config.format,
            &high_color_tex,
        );

        let dot_side_len = 2;
        let (compute_pipeline, compute_bind_group, compute_bind_group_layout, vertex_buffer) =
            Self::raytrace_shader(
                device,
                &lens_state.lens_bind_group_layout,
                &lens_state.params_bind_group_layout,
                dot_side_len,
                lens_state.ghost_indices.len() as u32,
            );

        let (triangulate_pipeline, tri_index_buffer) = Self::triangulate_shader(
            device,
            &compute_bind_group_layout,
            &lens_state.params_bind_group_layout,
            dot_side_len,
            lens_state.ghost_indices.len() as u32,
        );

        let convert_meta = std::fs::metadata("gpu/src/scenes/poly_tri/convert.wgsl").ok();
        let draw_meta = std::fs::metadata("gpu/src/scenes/poly_tri/draw.wgsl").ok();
        let compute_meta = std::fs::metadata("gpu/src/scenes/poly_tri/compute.wgsl").ok();
        let triangulate_meta = std::fs::metadata("gpu/src/scenes/poly_tri/triangulate.wgsl").ok();

        Self {
            tri_render_pipeline,
            tri_index_buffer,
            triangulate_pipeline,

            vertex_buffer,
            high_color_tex,
            conversion_render_pipeline,
            conversion_bind_group,
            convert_meta,
            draw_meta,
            compute_meta,
            triangulate_meta,
            format,
            conf_format: config.format,
            dot_side_len,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
        }
    }

    pub fn get_dots(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update_size: bool,
        lens_state: &LensState,
    ) -> Vec<polynomial_optics::DrawRay> {
        if update_size {
            // println!("update: {}", self.num_dots);
            let (compute_pipeline, compute_bind_group, compute_bind_group_layout, vertex_buffer) =
                Self::raytrace_shader(
                    device,
                    &lens_state.lens_bind_group_layout,
                    &lens_state.params_bind_group_layout,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.vertex_buffer = vertex_buffer;
            let (triangulate_pipeline, tri_index_buffer) = Self::triangulate_shader(
                device,
                &compute_bind_group_layout,
                &lens_state.params_bind_group_layout,
                self.dot_side_len,
                lens_state.ghost_indices.len() as u32,
            );
            self.triangulate_pipeline = triangulate_pipeline;
            self.tri_index_buffer = tri_index_buffer;
        }

        let num_ghosts = lens_state.ghost_indices.len() as u32;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Get dots Encoder"),
        });

        let work_group_count = std::cmp::min(
            (self.dot_side_len * self.dot_side_len * num_ghosts + 64 - 1) / 64,
            65535,
        ); // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.lens_bind_group, &[]);
            cpass.set_bind_group(2, &lens_state.params_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.triangulate_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }

        let output_buffer_size =
            (self.dot_side_len * self.dot_side_len * 32 * lens_state.ghost_indices.len() as u32)
                as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("Ray DST"),
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);
        encoder.copy_buffer_to_buffer(
            &self.vertex_buffer,
            0,
            &output_buffer,
            0,
            (self.dot_side_len * self.dot_side_len * 32 * lens_state.ghost_indices.len() as u32)
                .into(),
        );

        queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

        let mut out = vec![];
        device.poll(wgpu::Maintain::Wait);

        if let Ok(()) = pollster::block_on(buffer_future) {
            let data = buffer_slice.get_mapped_range();

            let vertices = unsafe { data.align_to::<f32>().1 };
            let vec_vertices = vertices.to_vec();
            let data = vec_vertices;

            for (i, elements) in data.chunks(8).enumerate() {
                let ray_num = i as u32 % (self.dot_side_len * self.dot_side_len);
                let ghost_num = i as u32 / (self.dot_side_len * self.dot_side_len);

                let ray_num_x =
                    (ray_num / (self.dot_side_len)) as f32 / (self.dot_side_len - 1) as f32;
                let ray_num_y =
                    (ray_num % (self.dot_side_len)) as f32 / (self.dot_side_len - 1) as f32;

                // print!("{:03}: ", i);
                let width = lens_state.pos_params[9];

                // println!(
                //     "ghost: {} num: {}, {:?}",
                //     ghost_num,
                //     ray_num,
                //     [
                //         ray_num_x as f32 * width - width / 2.,
                //         ray_num_y as f32 * width - width / 2.,
                //     ]
                // );
                out.push(polynomial_optics::DrawRay {
                    ghost_num,
                    init_pos: [
                        lens_state.pos_params[0],
                        lens_state.pos_params[1],
                        ray_num_x as f32 * width - width / 2.,
                        ray_num_y as f32 * width - width / 2.,
                    ],
                    pos: [elements[0], elements[1]],
                    aperture_pos: [elements[2], elements[3]],
                    entry_pos: [elements[4], elements[5]],
                    strength: elements[6],
                    wavelength: elements[7],
                });
                // println!("{:?}", out.last().unwrap());
            }
            // println!("{:?}", data);
            out
        } else {
            panic!("Failed to copy ray buffer!")
        }
    }

    pub fn update_dots(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        update_size: bool,
        lens_state: &LensState,
    ) {
        if update_size {
            // println!("update: {}", self.num_dots);
            let (compute_pipeline, compute_bind_group, compute_bind_group_layout, vertex_buffer) =
                Self::raytrace_shader(
                    device,
                    &lens_state.lens_bind_group_layout,
                    &lens_state.params_bind_group_layout,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.vertex_buffer = vertex_buffer;
            let (triangulate_pipeline, tri_index_buffer) = Self::triangulate_shader(
                device,
                &compute_bind_group_layout,
                &lens_state.params_bind_group_layout,
                self.dot_side_len,
                lens_state.ghost_indices.len() as u32,
            );
            self.triangulate_pipeline = triangulate_pipeline;
            self.tri_index_buffer = tri_index_buffer;
        }

        let num_ghosts = lens_state.ghost_indices.len() as u32;

        let work_group_count = std::cmp::min(
            (self.dot_side_len * self.dot_side_len * num_ghosts + 64 - 1) / 64,
            65535,
        ); // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.lens_bind_group, &[]);
            cpass.set_bind_group(2, &lens_state.params_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }

        let work_group_count = std::cmp::min(
            (self.dot_side_len * self.dot_side_len * num_ghosts + 64 - 1) / 64,
            65535,
        ); // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.triangulate_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            // cpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            // cpass.set_bind_group(2, &lens_state.lens_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }
    }
}

impl PolyTri {
    pub fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
    ) {
        // println!("size: {:?}, scale: {}", new_size.height * new_size.width * (scale_factor as u32) * (scale_factor as u32), scale_factor);
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

    pub fn update(&mut self, device: &wgpu::Device, lens_state: &LensState) {
        // if self.cell_timer.elapsed().unwrap().as_secs_f32() > 0.1 {
        //     self.cell_timer = SystemTime::now();
        //     self.update_cells(device, queue);
        // }
        //self.update_rays(device);
        if let Some(convert_meta) = &self.convert_meta {
            if convert_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/scenes/poly_tri/convert.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                print!("reloading convert shader ");
                let now = Instant::now();
                self.convert_meta = std::fs::metadata("gpu/src/scenes/poly_tri/convert.wgsl").ok();
                let (pipeline, bind_group) = Self::convert_shader(
                    device,
                    &lens_state.params_bind_group_layout,
                    &self.conf_format,
                    &self.high_color_tex,
                );
                self.conversion_render_pipeline = pipeline;
                self.conversion_bind_group = bind_group;
                println!("took {:?}.", now.elapsed());
            }
        }
        if let Some(draw_meta) = &self.draw_meta {
            if draw_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/scenes/poly_tri/draw.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                self.draw_meta = std::fs::metadata("gpu/src/scenes/poly_tri/draw.wgsl").ok();
                print!("reloading draw shader ");
                let now = Instant::now();
                let pipeline = Self::shader_draw(
                    device,
                    self.format,
                    &lens_state.params_bind_group_layout,
                    &lens_state.lens_bind_group_layout,
                );
                self.tri_render_pipeline = pipeline;
                println!("took {:?}.", now.elapsed());
            }
        }
        if let Some(compute_meta) = &self.compute_meta {
            if compute_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/scenes/poly_tri/compute.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                self.compute_meta = std::fs::metadata("gpu/src/scenes/poly_tri/compute.wgsl").ok();
                print!("reloading compute shader ");
                let now = Instant::now();
                let (pipeline, bind_group, compute_bind_group_layout, vertex_buffer) =
                    Self::raytrace_shader(
                        device,
                        &lens_state.lens_bind_group_layout,
                        &lens_state.params_bind_group_layout,
                        self.dot_side_len,
                        lens_state.ghost_indices.len() as u32,
                    );
                self.compute_pipeline = pipeline;
                self.compute_bind_group = bind_group;
                self.vertex_buffer = vertex_buffer;
                let (triangulate_pipeline, tri_index_buffer) = Self::triangulate_shader(
                    device,
                    &compute_bind_group_layout,
                    &lens_state.params_bind_group_layout,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
                self.triangulate_pipeline = triangulate_pipeline;
                self.tri_index_buffer = tri_index_buffer;
                println!("took {:?}.", now.elapsed());
            }
        }
        if let Some(triangulate_meta) = &self.triangulate_meta {
            if triangulate_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/scenes/poly_tri/triangulate.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                self.triangulate_meta =
                    std::fs::metadata("gpu/src/scenes/poly_tri/triangulate.wgsl").ok();
                print!("reloading triangulate shader ");
                let now = Instant::now();
                let (triangulate_pipeline, tri_index_buffer) = Self::triangulate_shader(
                    device,
                    &self.compute_bind_group_layout,
                    &lens_state.params_bind_group_layout,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
                self.triangulate_pipeline = triangulate_pipeline;
                self.tri_index_buffer = tri_index_buffer;
                println!("took {:?}.", now.elapsed());
            }
        }
    }

    /// retrace rays with wavelengths and ghosts then render
    pub fn render_color(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        lens_state: &mut LensState,
        update: bool,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut first = true;
        for wavelen in 0..lens_state.num_wavelengths {
            let start_wavelen = 0.38;
            let end_wavelen = 0.78;
            let wavelength = start_wavelen
                + wavelen as f64
                    * ((end_wavelen - start_wavelen) / lens_state.num_wavelengths as f64);
            // let strength = polynomial_optics::Lens::str_from_wavelen(wavelength) / 10.;

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            lens_state.pos_params[3] = wavelength as f32;
            queue.write_buffer(
                &lens_state.pos_params_buffer,
                0,
                bytemuck::cast_slice(&lens_state.pos_params),
            );
            // lens_state.pos_params[7] = strength as f32;

            self.update_dots(device, &mut encoder, update, lens_state);

            self.render_dots(
                &self.high_color_tex.view,
                &mut encoder,
                lens_state,
                first,
                lens_state.ghost_indices.len() as u32,
            );
            queue.submit(iter::once(encoder.finish()));
            first = false;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        if cfg!(debug_assertions) {
            let output_buffer_size = (self.dot_side_len
                * self.dot_side_len
                * 24
                * lens_state.ghost_indices.len() as u32)
                as wgpu::BufferAddress;
            let output_buffer_desc = wgpu::BufferDescriptor {
                size: output_buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                label: Some("Ray DST"),
                mapped_at_creation: false,
            };
            let output_buffer = device.create_buffer(&output_buffer_desc);
            encoder.copy_buffer_to_buffer(
                &self.vertex_buffer,
                0,
                &output_buffer,
                0,
                (self.dot_side_len
                    * self.dot_side_len
                    * 24
                    * lens_state.ghost_indices.len() as u32)
                    .into(),
            );

            self.convert(device, &mut encoder, view, lens_state);

            queue.submit(Some(encoder.finish()));

            let buffer_slice = output_buffer.slice(..);
            let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
            device.poll(wgpu::Maintain::Wait);

            if let Ok(()) = pollster::block_on(buffer_future) {
                let data = buffer_slice.get_mapped_range();

                let vertices = unsafe { data.align_to::<f32>().1 };
                let vec_vertices = vertices.to_vec();
                let data = vec_vertices;

                println!("----------------------------------------------------------------------------------");
                for (i, elements) in data.chunks(6).enumerate() {
                    print!("{:03}:", i);
                    print!("pos: {}, {}  ", elements[0], elements[1]);
                    print!("aper: {}, {}  ", elements[2], elements[3]);
                    print!("s: {}", elements[4]);
                    println!("w: {}", elements[5]);
                }
                // println!("{:?}", data);
            } else {
                panic!("Failed to copy ray buffer!")
            }
        } else {
            self.convert(device, &mut encoder, view, lens_state);
            queue.submit(iter::once(encoder.finish()));
        }

        Ok(())
    }

    /// retrace rays with wavelengths and ghosts then render
    pub fn render_hires(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        lens_state: &mut LensState,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut first = true;
        // let opacity = lens_state.opacity;
        // lens_state.opacity /= lens_state.num_wavelengths as f32;

        // lens_state.update(device, queue);
        for wavelen in 0..lens_state.num_wavelengths {
            let start_wavelen = 0.38;
            let end_wavelen = 0.78;
            let wavelength = start_wavelen
                + wavelen as f64
                    * ((end_wavelen - start_wavelen) / lens_state.num_wavelengths as f64);
            // let strength = polynomial_optics::Lens::str_from_wavelen(wavelength) / 10.;

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            lens_state.pos_params[3] = wavelength as f32;
            queue.write_buffer(
                &lens_state.pos_params_buffer,
                0,
                bytemuck::cast_slice(&lens_state.pos_params),
            );
            // lens_state.pos_params[7] = strength as f32;

            self.update_dots(device, &mut encoder, true, lens_state);

            self.render_dots(
                &self.high_color_tex.view,
                &mut encoder,
                lens_state,
                first,
                lens_state.ghost_indices.len() as u32,
            );
            queue.submit(iter::once(encoder.finish()));
            device.poll(wgpu::Maintain::Wait);
            first = false;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.convert(device, &mut encoder, view, lens_state);
        queue.submit(iter::once(encoder.finish()));

        // lens_state.opacity = opacity;
        // lens_state.update(device, queue);
        Ok(())
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

        self.render_dots(&self.high_color_tex.view, &mut encoder, lens_state, true, 1);

        self.convert(device, &mut encoder, view, lens_state);

        queue.submit(iter::once(encoder.finish()));

        // conversion pass

        Ok(())
    }

    pub fn render_dots(
        &self,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        lens_state: &LensState,
        clear: bool,
        num_ghosts: u32,
    ) {
        // create render pass descriptor and its color attachments
        let color_attachments = [wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: if clear {
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
            rpass.set_pipeline(&self.tri_render_pipeline);
            rpass.set_bind_group(0, &lens_state.lens_bind_group, &[]);
            rpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            // the three instance-local vertices
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.tri_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw_indexed(0..self.get_num_tris() * num_ghosts * 6, 0, 0..1);
            // rpass.draw(0..self.dot_side_len * self.dot_side_len, 0..1);
        }
    }

    pub fn convert(
        &self,
        device: &wgpu::Device,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        lens_state: &LensState,
    ) {
        let vertex_buffer_data = [
            -1.0f32, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0,
        ];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&vertex_buffer_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // create render pass descriptor and its color attachments
        let color_attachments = [wgpu::RenderPassColorAttachment {
            view,
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

        {
            // render pass
            let mut rpass = encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.conversion_render_pipeline);
            rpass.set_bind_group(0, &self.conversion_bind_group, &[]);
            rpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            rpass.set_vertex_buffer(0, vertices_buffer.slice(..));
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw(0..vertex_buffer_data.len() as u32 / 2, 0..1);
        }
    }
}
