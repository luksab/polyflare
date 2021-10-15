use wgpu::TextureView;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::demo3d::Demo3d;
use crate::game_of_life::GameOfLife;
use crate::scene::Scene;

pub struct State {
    pub window: Window,
    pub surface: wgpu::Surface,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub scale_factor: f64,
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    scenes: Vec<Box<dyn Scene>>,
}

impl State {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
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
                        max_storage_buffer_binding_size: 128 << 20, // default
                        max_vertex_buffers: 8,          // default
                        max_vertex_attributes: 16,      // default
                        max_vertex_buffer_array_stride: 2048, // default
                        max_push_constant_size: 0,
                        min_uniform_buffer_offset_alignment: 256,
                        min_storage_buffer_offset_alignment: 256, // default
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let mut scenes: Vec<Box<dyn Scene>> = Vec::new();

        let game_of_life = GameOfLife::new(&device, &config).await;
        scenes.push(Box::new(game_of_life));

        let demo = Demo3d::new(&device, &queue, &config).await;

        scenes.push(Box::new(demo));

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
            scenes,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: Option<&f64>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
        for scene in &mut self.scenes {
            scene.resize(
                self.size,
                *scale_factor.unwrap_or(&self.scale_factor),
                &self.device,
                &self.config,
                &self.queue,
            )
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        for scene in &mut self.scenes {
            scene.input(event);
        }
        match event {
            _ => false,
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        for scene in &mut self.scenes {
            scene.update(dt, &self.device, &self.queue);
        }
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        depth_view: Option<&wgpu::TextureView>,
        index: usize,
    ) -> Result<(), wgpu::SurfaceError> {
        self.scenes
            .get_mut(index)
            .expect("scene not found!")
            .render(view, depth_view, &self.device, &self.queue)
    }
}
