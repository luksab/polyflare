use gpu::*;

pub struct Gpu {
    pub state: State,
}

impl Gpu {
    pub fn new() -> Gpu {
        let mut state = pollster::block_on(State::new(
            wgpu::Backends::PRIMARY,
            true,
            None,
            [1920, 1080],
        ));
        Gpu { state }
    }
}

/// # The main struct of this framework
/// mainly defers actual work to scenes
pub struct State {
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,
    pub size: [u32; 2],
}

impl State {
    /// create a new state, does not create any scenes
    ///
    /// add those by calling `state.scenes.push(scene)`
    ///
    /// backend is one of  `VULKAN, GL, METAL, DX11, DX12, BROWSER_WEBGPU, PRIMARY`
    pub async fn new(
        backend: wgpu::Backends,
        low_req: bool,
        adapter: Option<usize>,
        size: [u32; 2],
    ) -> Self {
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(backend);
        instance
            .enumerate_adapters(backend)
            .enumerate()
            .for_each(|adapter| {
                println!("{:?}", adapter);
            });

        let adapter = match adapter {
            Some(adapter_num) => instance
                .enumerate_adapters(backend)
                .nth(adapter_num)
                .unwrap(),
            None => instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect("no adapter compatible with surface found"),
        };
        println!("\nadapter: {:?}", adapter.get_info());
        println!("limits: {:#?}", adapter.limits());
        let limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits,
                },
                None, // Trace path
            )
            .await
            .expect("even the lowest requirements device is not available");
        /*match low_req {
            true => ,
            false => adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits {
                            max_texture_dimension_1d: 16384,
                            max_texture_dimension_2d: 16384,
                            max_texture_dimension_3d: 2048,
                            max_texture_array_layers: 256, // default
                            max_bind_groups: 4,             // default
                            max_dynamic_uniform_buffers_per_pipeline_layout: 8, // default
                            max_dynamic_storage_buffers_per_pipeline_layout: 4, // default
                            max_sampled_textures_per_shader_stage: 16, // default
                            max_samplers_per_shader_stage: 16, // default
                            max_storage_buffers_per_shader_stage: 4, // default
                            max_storage_textures_per_shader_stage: 4, // default
                            max_uniform_buffers_per_shader_stage: 12, // default
                            max_uniform_buffer_binding_size: 16384, // default
                            max_storage_buffer_binding_size: u32::MAX, // 128 << 20, // default
                            max_vertex_buffers: 8,          // default
                            max_vertex_attributes: 16,      // default
                            max_vertex_buffer_array_stride: 2048, // default
                            max_push_constant_size: 0,      // default
                            min_uniform_buffer_offset_alignment: 256, // default
                            min_storage_buffer_offset_alignment: 256, // default
                            max_inter_stage_shader_components: 60, // default
                            max_compute_workgroup_storage_size: 16352, // default
                            max_compute_invocations_per_workgroup: 256, // default
                            max_compute_workgroup_size_x: 256, // default
                            max_compute_workgroup_size_y: 256, // default
                            max_compute_workgroup_size_z: 64, // default
                            max_compute_workgroups_per_dimension: 65535, // default
                        },
                    },
                    None, // Trace path
                )
                .await
                .expect("failed to create device with high requirement"),
        };*/

        Self {
            device,
            adapter,
            queue,
            size,
        }
    }

    /// call this when the main window is resized.
    ///
    /// internally calls resize on all scenes
    pub fn resize(&mut self, new_size: [u32; 2]) {
        // check that the size is real
        if new_size[0] > 0 && new_size[1] > 0 {
            self.size = new_size;
            // self.surface.configure(&self.device, &self.config);
            todo!();
        }
    }
}

