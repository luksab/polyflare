use std::{mem, time::SystemTime};

use std::iter;

use wgpu::{util::DeviceExt, Queue, SurfaceConfiguration, TextureView};

use crate::scene::Scene;

// number of boid particles to simulate
const SIDE_LEN: u32 = 256;

// number of single-particle calculations (invocations) in each gpu work group
const CELLS_PER_GROUP: u32 = 64;

pub struct GameOfLife {
    boid_render_pipeline: wgpu::RenderPipeline,
    vertices_buffer: wgpu::Buffer,

    particle_bind_groups: Vec<wgpu::BindGroup>,
    render_bind_groups: Vec<wgpu::BindGroup>,
    sim_param_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    work_group_count: u32,
    frame_num: usize,
    cell_timer: SystemTime,
}

impl GameOfLife {
    pub async fn new(device: &wgpu::Device, config: &SurfaceConfiguration) -> Self {
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into()),
        });

        // buffer for simulation parameters uniform
        let sim_param_data = [SIDE_LEN, 512, 512].to_vec();
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Parameter Buffer"),
            contents: bytemuck::cast_slice(&sim_param_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
                                (sim_param_data.len() * mem::size_of::<u32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((2 * SIDE_LEN * SIDE_LEN) as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((2 * SIDE_LEN * SIDE_LEN) as _),
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

        // create render pipeline with simProps as bind group layout

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_param_data.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new((2 * SIDE_LEN * SIDE_LEN) as _),
                        },
                        count: None,
                    },
                ],
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
                    // array_stride: 2 * 4,
                    // step_mode: wgpu::VertexStepMode::Vertex,
                    // attributes: &wgpu::vertex_attr_array![2 => Float32x2],
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
                module: &draw_shader,
                entry_point: "main",
                targets: &[config.format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        // create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // buffer for all particles data of type [bool,...]
        let mut initial_particle_data = vec![0 as u32; (2 * SIDE_LEN * SIDE_LEN) as usize];
        for (i, particle_instance_chunk) in &mut initial_particle_data.chunks_mut(2).enumerate() {
            particle_instance_chunk[0] = i as u32; // bool??
            particle_instance_chunk[1] = fastrand::u32(0..6) / 5; // bool??
        }

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            particle_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Particle Buffer {}", i)),
                    contents: bytemuck::cast_slice(&initial_particle_data),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst
        for i in 0..2 {
            particle_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particle_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                ],
                label: None,
            }));
        }

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        let work_group_count =
            (((SIDE_LEN * SIDE_LEN) as f32) / (CELLS_PER_GROUP as f32)).ceil() as u32;

        let vertex_buffer_data = [
            -0.1f32, -0.1, 0.1, -0.1, -0.1, 0.1, -0.1, 0.1, 0.1, 0.1, 0.1, -0.1,
        ];
        // let vertex_buffer_data = [-1.0f32, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&vertex_buffer_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut render_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            render_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &render_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particle_buffers[i].as_entire_binding(),
                    },
                ],
                label: None,
            }));
        }

        Self {
            boid_render_pipeline,

            particle_bind_groups,
            sim_param_buffer,
            compute_pipeline,
            work_group_count,
            frame_num: 0,
            cell_timer: SystemTime::now(),
            vertices_buffer,
            render_bind_groups,
        }
    }

    fn update_cells(&mut self, device: &wgpu::Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch(self.work_group_count, 1, 1);
        }
        queue.submit(iter::once(encoder.finish()));

        // update frame count
        self.frame_num += 1;
    }
}

impl Scene for GameOfLife {
    fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        _device: &wgpu::Device,
        _config: &SurfaceConfiguration,
        queue: &Queue,
    ) {
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&[
                SIDE_LEN,
                (new_size.width as f32 * scale_factor as f32) as u32,
                (new_size.height as f32 * scale_factor as f32) as u32,
            ]),
        );
    }

    fn input(&mut self, _event: &winit::event::WindowEvent) -> bool {
        true
    }

    fn update(&mut self, _dt: std::time::Duration, device: &wgpu::Device, queue: &Queue) {
        if self.cell_timer.elapsed().unwrap().as_secs_f32() > 0.1 {
            self.cell_timer = SystemTime::now();
            self.update_cells(device, queue);
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
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
            rpass.set_pipeline(&self.boid_render_pipeline);
            //rpass.set_bind_group(1, &self.particle_bind_groups[self.frame_num % 2], &[]);
            // the three instance-local vertices
            rpass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.set_bind_group(0, &self.render_bind_groups[self.frame_num % 2], &[]);
            rpass.draw(0..6, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));

        Ok(())
    }
}
