use std::{fs::read_to_string, iter, mem};

use cgmath::{InnerSpace, Vector3};
use wgpu::{
    util::DeviceExt, BindGroup, Buffer, Queue, RenderPipeline, SurfaceConfiguration, TextureFormat,
    TextureView,
};

use crate::{scene::Scene, texture::Texture};

use polynomial_optics::*;

pub struct PolyOptics {
    boid_render_pipeline: wgpu::RenderPipeline,
    high_color_tex: Texture,
    conversion_render_pipeline: wgpu::RenderPipeline,
    conversion_bind_group: wgpu::BindGroup,
    vertices_buffer: wgpu::Buffer,

    // particle_bind_groups: Vec<wgpu::BindGroup>,
    render_bind_group: wgpu::BindGroup,
    sim_param_buffer: wgpu::Buffer,
    pub sim_params: [f32; 5],
    // compute_pipeline: wgpu::ComputePipeline,
    // work_group_count: u32,
    // frame_num: usize,
    // cell_timer: SystemTime,
    pub lens: Lens,
    rays: Vec<f32>,
    pub num_rays: u32,
    pub draw_mode: u32,
    pub center_pos: Vector3<f64>,
    pub direction: Vector3<f64>,

    convert_meta: std::fs::Metadata,
    draw_meta: std::fs::Metadata,
    format: TextureFormat,
    confFormat: TextureFormat,
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
                    array_stride: 3 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 2 * 4,
                            shader_location: 1,
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

    pub async fn new(device: &wgpu::Device, config: &SurfaceConfiguration) -> Self {
        // let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        //     label: None,
        //     source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        // });

        let format = wgpu::TextureFormat::Rgba16Float;
        let high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        // let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //     label: Some("Conversion Pipeline Layout"),
        //     bind_group_layouts: &[&conversion_bind_group_layout],
        //     push_constant_ranges: &[],
        // });

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
            let space = Element::Space(0.5);
            let radius = 3.0;
            let lens_entry = Element::SphericalLensEntry {
                radius,
                glass: Glass {
                    ior: 1.5,
                    coating: (),
                },
                position: -2.0,
            };
            let lens_exit_pos = 1.0;
            let lens_exit = Element::SphericalLensExit {
                radius,
                glass: Glass {
                    ior: 1.5,
                    coating: (),
                },
                position: lens_exit_pos,
            };
            // line.width = 3.0;
            // // lens entry
            // line.draw_circle(&mut pixmap, -radius as f32 - 2.0, 0., radius as f32);

            // // lens exit
            // line.color = Color::from_rgba8(127, 127, 127, 255);
            // line.draw_circle(
            //     &mut pixmap,
            //     (-3.) * radius as f32 + lens_exit_pos as f32,
            //     0.,
            //     radius as f32,
            // );
            // line.width = 0.1;

            println!("space: {:?}", space);
            println!("lens: {:?}", lens_entry);
            //println!("ray: {:?}", ray);

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

        let rays = vec![0.0, 0.0];
        // let vertex_buffer_data = [-1.0f32, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&rays[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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

        let convert_meta = std::fs::metadata("gpu/src/scenes/poly_optics/convert.wgsl").unwrap();
        let draw_meta = std::fs::metadata("gpu/src/scenes/poly_optics/draw.wgsl").unwrap();

        Self {
            boid_render_pipeline,

            sim_param_buffer,
            sim_params,
            vertices_buffer,
            render_bind_group,
            lens,

            draw_mode: 2,
            num_rays: 256,
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
            convert_meta,
            draw_meta,
            format,
            confFormat: config.format,
        }
    }

    pub fn write_buffer(&self, queue: &Queue) {
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );
    }

    pub fn update_rays(&mut self, device: &wgpu::Device) {
        self.rays = self.lens.get_rays(
            self.num_rays,
            self.center_pos,
            self.direction,
            self.draw_mode,
        );
        self.vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.rays[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
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

        // // update frame count
        // self.frame_num += 1;
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
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );

        let format = wgpu::TextureFormat::Rgba16Float;
        self.high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

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
                &self.confFormat,
                &self.high_color_tex,
            );
            self.conversion_render_pipeline = pipeline;
            self.conversion_bind_group = bind_group;
        }

        if self.draw_meta.modified().unwrap()
            != std::fs::metadata("gpu/src/scenes/poly_optics/draw.wgsl")
                .unwrap()
                .modified()
                .unwrap()
        {
            let (pipeline, bind_group) = Self::shader_draw(
                device,
                &self.sim_params,
                &self.sim_param_buffer,
                self.format,
            );
            self.conversion_render_pipeline = pipeline;
            self.conversion_bind_group = bind_group;
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
            rpass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw(
                0..if self.draw_mode & 4 == 0 {
                    self.rays.len() as u32 / 3
                } else {
                    0
                },
                0..1,
            );
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
}
