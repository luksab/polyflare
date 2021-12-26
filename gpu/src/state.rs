use wgpu::Backends;
use winit::event_loop::EventLoop;
use winit::window::Window;

/// # The main struct of this framework
/// mainly defers actual work to scenes
pub struct State {
    pub window: Window,
    pub surface: wgpu::Surface,
    pub size: winit::dpi::PhysicalSize<u32>,
    /// scaling for high dpi
    pub scale_factor: f64,
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,
    /// `SurfaceConfiguration` of the main window
    pub config: wgpu::SurfaceConfiguration,
}

impl State {
    /// create a new state, does not create any scenes
    ///
    /// add those by calling `state.scenes.push(scene)`
    ///
    /// backend is one of  `VULKAN, GL, METAL, DX11, DX12, BROWSER_WEBGPU, PRIMARY`
    pub async fn new(event_loop: &EventLoop<()>, backend: Backends, low_req: bool) -> Self {
        // get the title at compile time from env
        let title = env!("CARGO_PKG_NAME");
        let window = winit::window::WindowBuilder::new()
            .with_title(title)
            .build(event_loop)
            .unwrap();
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = match low_req {
            true => adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits {
                            max_texture_dimension_1d: 4096,
                            max_texture_dimension_2d: 4096,
                            max_texture_dimension_3d: 2048,
                            max_texture_array_layers: 2048, // default
                            max_bind_groups: 4,             // default
                            max_dynamic_uniform_buffers_per_pipeline_layout: 8, // default
                            max_dynamic_storage_buffers_per_pipeline_layout: 4, // default
                            max_sampled_textures_per_shader_stage: 16, // default
                            max_samplers_per_shader_stage: 16, // default
                            max_storage_buffers_per_shader_stage: 4, // default
                            max_storage_textures_per_shader_stage: 4, // default
                            max_uniform_buffers_per_shader_stage: 12, // default
                            max_uniform_buffer_binding_size: 16384, // default
                            max_storage_buffer_binding_size: 128 << 20, // 128 << 20, // default
                            max_vertex_buffers: 8,          // default
                            max_vertex_attributes: 16,      // default
                            max_vertex_buffer_array_stride: 2048, // default
                            max_push_constant_size: 0,
                            min_uniform_buffer_offset_alignment: 256, // default
                            min_storage_buffer_offset_alignment: 256, // default
                        },
                    },
                    None, // Trace path
                )
                .await
                .unwrap(),
            false => adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: wgpu::Features::empty(),
                        limits: wgpu::Limits {
                            max_texture_dimension_1d: 4096,
                            max_texture_dimension_2d: 4096,
                            max_texture_dimension_3d: 2048,
                            max_texture_array_layers: 2048, // default
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
                            max_push_constant_size: 0,
                            min_uniform_buffer_offset_alignment: 256, // default
                            min_storage_buffer_offset_alignment: 256, // default
                        },
                    },
                    None, // Trace path
                )
                .await
                .unwrap(),
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let scale_factor = window.scale_factor();
        Self {
            window,
            surface,
            device,
            adapter,
            queue,
            config,

            size,
            scale_factor,
        }
    }

    /// call this when the main window is resized.
    ///
    /// internally calls resize on all scenes
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, _scale_factor: Option<&f64>) {
        // check that the size is real
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}
