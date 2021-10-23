use std::{iter, mem};

use cgmath::{InnerSpace, Vector3};
use wgpu::{util::DeviceExt, Queue, SurfaceConfiguration, TextureView};

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
}

impl PolyOptics {
    pub async fn new(device: &wgpu::Device, config: &SurfaceConfiguration) -> Self {
        // let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        //     label: None,
        //     source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        // });
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("polyOptics"),
            source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into()),
        });

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

        // // create compute bind layout group and compute pipeline layout
        // let compute_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         entries: &[
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::COMPUTE,
        //                 ty: wgpu::BindingType::Buffer {
        //                     ty: wgpu::BufferBindingType::Uniform,
        //                     has_dynamic_offset: false,
        //                     min_binding_size: wgpu::BufferSize::new(
        //                         (sim_param_data.len() * mem::size_of::<u32>()) as _,
        //                     ),
        //                 },
        //                 count: None,
        //             },
        //             // TODO: BufferSize
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 1,
        //                 visibility: wgpu::ShaderStages::COMPUTE,
        //                 ty: wgpu::BindingType::Buffer {
        //                     ty: wgpu::BufferBindingType::Storage { read_only: true },
        //                     has_dynamic_offset: false,
        //                     min_binding_size: wgpu::BufferSize::new((2 * NUM_RAYS * NUM_RAYS) as _),
        //                 },
        //                 count: None,
        //             },
        //             // TODO: BufferSize
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 2,
        //                 visibility: wgpu::ShaderStages::COMPUTE,
        //                 ty: wgpu::BindingType::Buffer {
        //                     ty: wgpu::BufferBindingType::Storage { read_only: false },
        //                     has_dynamic_offset: false,
        //                     min_binding_size: wgpu::BufferSize::new((2 * NUM_RAYS * NUM_RAYS) as _),
        //                 },
        //                 count: None,
        //             },
        //         ],
        //         label: None,
        //     });
        // let compute_pipeline_layout =
        //     device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //         label: Some("compute"),
        //         bind_group_layouts: &[&compute_bind_group_layout],
        //         push_constant_ranges: &[],
        //     });

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
                source: wgpu::ShaderSource::Wgsl(include_str!("convert.wgsl").into()),
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
                        format: config.format,
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

        // create render pipeline with simProps as bind group layout

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

        // // create compute pipeline
        // let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        //     label: Some("Compute pipeline"),
        //     layout: Some(&compute_pipeline_layout),
        //     module: &compute_shader,
        //     entry_point: "main",
        // });

        // buffer for all particles data of type [bool,...]
        // TODO: BufferSize
        // let mut initial_particle_data = vec![0 as u32; (2 * NUM_RAYS * NUM_RAYS) as usize];
        // for (i, particle_instance_chunk) in &mut initial_particle_data.chunks_mut(2).enumerate() {
        //     particle_instance_chunk[0] = i as u32; // bool??
        //     particle_instance_chunk[1] = fastrand::u32(0..6) / 5; // bool??
        // }

        // let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        // let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        // for i in 0..2 {
        //     particle_buffers.push(
        //         device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //             label: Some(&format!("Particle Buffer {}", i)),
        //             contents: bytemuck::cast_slice(&initial_particle_data),
        //             usage: wgpu::BufferUsages::VERTEX
        //                 | wgpu::BufferUsages::STORAGE
        //                 | wgpu::BufferUsages::COPY_DST,
        //         }),
        //     );
        // }

        // // create two bind groups, one for each buffer as the src
        // // where the alternate buffer is used as the dst
        // for i in 0..2 {
        //     particle_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
        //         layout: &compute_bind_group_layout,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: sim_param_buffer.as_entire_binding(),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: particle_buffers[i].as_entire_binding(),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 2,
        //                 resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
        //             },
        //         ],
        //         label: None,
        //     }));
        // }

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        // TODO: BufferSize
        // let work_group_count =
        //     (((NUM_RAYS * NUM_RAYS) as f32) / (RAYS_PER_GROUP as f32)).ceil() as u32;

        // let vertex_buffer_data = [
        //     -0.1f32, -0.1, 0.1, -0.1, -0.1, 0.1, -0.1, 0.1, 0.1, 0.1, 0.1, -0.1,
        // ];

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
        let rays = vec![0.0, 0.0];
        // let vertex_buffer_data = [-1.0f32, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&rays[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sim_param_buffer.as_entire_binding(),
            }],
            label: None,
        });

        Self {
            boid_render_pipeline,

            // particle_bind_groups,
            sim_param_buffer,
            sim_params,
            // compute_pipeline,
            // work_group_count,
            // frame_num: 0,
            // cell_timer: SystemTime::now(),
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
    }

    fn input(&mut self, _event: &winit::event::WindowEvent) -> bool {
        true
    }

    fn update(&mut self, _dt: std::time::Duration, _device: &wgpu::Device, _queue: &Queue) {
        // if self.cell_timer.elapsed().unwrap().as_secs_f32() > 0.1 {
        //     self.cell_timer = SystemTime::now();
        //     self.update_cells(device, queue);
        // }
        //self.update_rays(device);
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
                    r: 0.2,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
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
                        r: 0.2,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
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
