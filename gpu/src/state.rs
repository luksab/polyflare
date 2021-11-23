use std::rc::Rc;
use std::sync::Mutex;

use wgpu::{Backends, TextureView};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::scene::Scene;
/// # The main struct of this framework
/// mainly defers actual work to scenes
pub struct State {
    pub window: Window,
    pub surface: wgpu::Surface,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub scale_factor: f64,
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    /// the scenes saved in the state struct
    pub scenes: Vec<Rc<Mutex<dyn Scene>>>,
}

impl State {
    /// create a new state, does not create any scenes
    ///
    /// add those by calling `state.scenes.push(scene)`
    pub async fn new(event_loop: &EventLoop<()>, backend: Backends) -> Self {
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
                        max_storage_buffer_binding_size: u32::MAX, // default
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
            .unwrap();

        println!("{:?}", surface.get_preferred_format(&adapter).unwrap());

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let scenes: Vec<Rc<Mutex<dyn Scene>>> = Vec::new();

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

    /// call this when the window is resized.
    ///
    /// internally calls resize on all scenes
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: Option<&f64>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
        for scene in &mut self.scenes {
            scene.lock().unwrap().resize(
                self.size,
                *scale_factor.unwrap_or(&self.scale_factor),
                &self.device,
                &self.config,
                &self.queue,
            )
        }
    }

    /// let all scenes handle a WindowEvent
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        for scene in &mut self.scenes {
            scene.lock().unwrap().input(event);
        }
        match event {
            _ => false,
        }
    }

    /// update all scenes - call this once a frame
    pub fn update(&mut self, dt: std::time::Duration) {
        for scene in &mut self.scenes {
            scene.lock().unwrap().update(dt, &self.device, &self.queue);
        }
    }

    /// render the scene at index `index`
    pub fn render(
        &mut self,
        view: &TextureView,
        depth_view: Option<&wgpu::TextureView>,
        index: usize,
    ) -> Result<(), wgpu::SurfaceError> {
        self.scenes
            .get_mut(index)
            .expect("scene not found!")
            .lock()
            .unwrap()
            .render(view, depth_view, &self.device, &self.queue)
    }
}
