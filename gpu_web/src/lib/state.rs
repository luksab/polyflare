use crate::console_log;
use std::borrow::Cow;
use web_sys::console;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

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
    pub async fn new(
        event_loop: &EventLoop<()>,
        backend: wgpu::Backends,
        _low_req: bool,
        adapter: Option<usize>,
        disable_vsync: bool,
    ) -> Self {
        // get the title at compile time from env
        let title = env!("CARGO_PKG_NAME");
        // let window = winit::window::WindowBuilder::new()
        //     .with_title(title)
        //     .build(event_loop)
        //     .unwrap();
        // let window = web_sys::window().unwrap();
        let window = winit::window::Window::new(&event_loop).unwrap();
        let size = window.inner_size();

        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        console_log("hiii");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&web_sys::Element::from(window.canvas()))
            .expect("couldn't append canvas to document body");
        // wasm_bindgen_futures::spawn_local(run(event_loop, window));

        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface
                .get_preferred_format(&adapter)
                .expect("surface not compatible with gpu"),
            width: size.width,
            height: size.height,
            present_mode: if disable_vsync {
                wgpu::PresentMode::Immediate
            } else {
                wgpu::PresentMode::Fifo
            },
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
