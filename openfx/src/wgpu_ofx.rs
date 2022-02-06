use std::iter;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

use gpu::lens_state::LensState;
use wgpu::Device;
use wgpu::Queue;
use wgpu::SurfaceConfiguration;

use gpu::scenes::PolyRes;
use gpu::scenes::PolyTri;
use wgpu::Texture;

use half::f16;

pub struct Gpu {
    pub state: State,
    pub lens_ui: LensState,
    pub poly_res: PolyRes,
    pub poly_tri: PolyTri,
    pub raw: Arc<RwLock<Vec<Vec<(f32, f32, f32, f32)>>>>,
}

impl Gpu {
    pub fn new() -> Gpu {
        let mut state = pollster::block_on(State::new(wgpu::Backends::PRIMARY, None, [1920, 1080]));
        let mut lens_ui = LensState::default(&state.device);
        lens_ui.init(&state.device, &state.queue);
        let mut poly_res = pollster::block_on(PolyRes::new(&state.device, &state.config, &lens_ui));
        let mut poly_tri = pollster::block_on(PolyTri::new(&state.device, &state.config, &lens_ui));

        Gpu {
            state,
            lens_ui,
            poly_res,
            poly_tri,
            raw: Arc::new(RwLock::new(vec![])),
        }
    }
}

/// # The main struct of this framework
/// mainly defers actual work to scenes
pub struct State {
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,
    pub config: SurfaceConfiguration,
}

impl State {
    /// create a new state, does not create any scenes
    ///
    /// add those by calling `state.scenes.push(scene)`
    ///
    /// backend is one of  `VULKAN, GL, METAL, DX11, DX12, BROWSER_WEBGPU, PRIMARY`
    pub async fn new(backend: wgpu::Backends, adapter: Option<usize>, size: [u32; 2]) -> Self {
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

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba16Float,
            width: size[0],
            height: size[1],
            present_mode: wgpu::PresentMode::Immediate,
        };

        Self {
            device,
            adapter,
            queue,
            config,
        }
    }

    pub fn resize(&mut self, new_size: [u32; 2]) {
        // check that the size is real
        if new_size[0] > 0 && new_size[1] > 0 {
            self.config.width = new_size[0];
            self.config.height = new_size[1];
            // self.surface.configure(&self.device, &self.config);
        }
    }
}

impl Gpu {
    /// call this when the main window is resized.
    ///
    /// internally calls resize on all scenes
    pub fn resize(&mut self, new_size: [u32; 2]) {
        // check that the size is real
        if new_size[0] > 0 && new_size[1] > 0 {
            // self.surface.configure(&self.device, &self.config);
            self.state.resize(new_size);
            self.lens_ui.resize_window(new_size, 1.0);

            self.poly_res
                .resize(new_size, 1.0, &self.state.device, &self.state.config);

            self.poly_tri
                .resize(new_size, 1.0, &self.state.device, &self.state.config);
        }
    }

    pub fn update(
        &mut self,
        parameters: Result<(f64, f64, f64, f64, bool, f64, f64, f64), ofx::Error>,
    ) {
        //TODO: todo!("add update logic");
        // let (update_lens, update_ray_num, update_dot_num, render, update_res, compute) = self
        //     .lens_ui
        //     .build_ui(&ui, &self.state.device, &self.state.queue);

        let (
            dots_exponent,
            num_wavelengths,
            opacity,
            scale_fact,
            triangulate,
            pos_x_param,
            pos_y_param,
            pos_z_param,
        ) = parameters.unwrap();
        println!(
            "dots_exponent: {}, num_wavelengths: {}, opacity: {}, scale_fact: {}, triangulate: {}, pos_x_param: {}, pos_y_param: {}, pos_z_param: {}",
            dots_exponent, num_wavelengths, opacity, scale_fact, triangulate, pos_x_param, pos_y_param, pos_z_param
        );

        self.poly_res.num_dots = 10.0_f64.powf(self.lens_ui.dots_exponent) as u32;
        self.poly_tri.dot_side_len = 10.0_f64.powf(self.lens_ui.dots_exponent).sqrt() as u32;
        self.lens_ui.sim_params[11] = self.poly_tri.dot_side_len as f32;

        self.lens_ui.sim_params[12] = scale_fact as f32;
        self.lens_ui.opacity = (opacity * (33. / self.poly_tri.dot_side_len as f64)) as f32;
        self.lens_ui.dots_exponent = dots_exponent;
        self.lens_ui.num_wavelengths = num_wavelengths as u32;
        self.lens_ui.triangulate = triangulate;
        self.lens_ui.pos_params[0] = pos_x_param as f32;
        self.lens_ui.pos_params[1] = pos_y_param as f32;
        self.lens_ui.pos_params[2] = pos_z_param as f32;

        self.lens_ui.needs_update = true;
        self.lens_ui.update(&self.state.device, &self.state.queue);

        // if self.lens_ui.triangulate {
        //     self.poly_tri.update(&self.state.device, &self.lens_ui);
        // } else {
        //     self.poly_res.update(&self.state.device, &self.lens_ui);
        // }
    }

    fn tex_to_raw(
        tex: &Texture,
        size: [u32; 2],
        device: &Device,
        queue: &Queue,
    ) -> Vec<Vec<(f32, f32, f32, f32)>> {
        let output_buffer_size = (size[0] * size[1] * 8) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("Ray DST"),
            mapped_at_creation: false,
        };

        let output_buffer = device.create_buffer(&output_buffer_desc);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        let texture_extent = wgpu::Extent3d {
            width: size[0] as u32,
            height: size[1] as u32,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(std::num::NonZeroU32::new(size[0] * 8).unwrap()),
                    rows_per_image: None,
                },
            },
            texture_extent,
        );

        queue.submit(iter::once(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);

        if let Ok(()) = pollster::block_on(buffer_future) {
            let data = buffer_slice.get_mapped_range().to_vec();
            let data: Vec<f32> = data.chunks(2).map(|chunk| {
                f16::from_le_bytes([chunk[0], chunk[1]]).to_f32()
            }).collect();
            let data: Vec<(f32, f32, f32, f32)> = data
                .chunks(4)
                .map(|chunk| (chunk[0], chunk[1], chunk[2], chunk[3]))
                .collect();
            let data: Vec<Vec<(f32, f32, f32, f32)>> =
                data.chunks(size[0] as usize).map(|x| x.to_vec()).collect();
            return data;
        } else {
            panic!("Failed to copy texture to CPU!")
        }
    }

    pub fn render(&mut self) {
        let now = Instant::now();
        let size = [self.state.config.width, self.state.config.height]; //[2048, 2048];
        println!("size: {:?}", size);
        let extend = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("hi-res"),
            size: extend,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,//wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        };
        let tex = self.state.device.create_texture(&desc);

        if self.lens_ui.triangulate {
            self.poly_tri.resize(
                [size[0], size[1]],
                1.0,
                &self.state.device,
                &self.state.config,
            );
        } else {
            self.poly_res.resize(
                [size[0], size[1]],
                1.0,
                &self.state.device,
                &self.state.config,
            );
        }

        let sim_params = self.lens_ui.sim_params;

        self.lens_ui.resize_window([size[0], size[1]], 1.0);

        self.lens_ui.num_wavelengths *= 10;
        self.lens_ui.update(&self.state.device, &self.state.queue);

        if self.lens_ui.triangulate {
            self.poly_tri
                .render_hires(
                    &tex.create_view(&wgpu::TextureViewDescriptor::default()),
                    &self.state.device,
                    &self.state.queue,
                    &mut self.lens_ui,
                )
                .unwrap();

            self.poly_tri.resize(
                [sim_params[9] as _, sim_params[10] as _],
                2.0,
                &self.state.device,
                &self.state.config,
            );
        } else {
            self.poly_res.num_dots = 5_000_000; // to scale opactity correctly
            self.poly_res
                .render_hires(
                    &tex.create_view(&wgpu::TextureViewDescriptor::default()),
                    &self.state.device,
                    &self.state.queue,
                    10.0_f64.powf(self.lens_ui.dots_exponent + 5.) as u64,
                    &mut self.lens_ui,
                )
                .unwrap();

            self.poly_res.resize(
                [sim_params[9] as _, sim_params[10] as _],
                1.0,
                &self.state.device,
                &self.state.config,
            );
        }
        self.lens_ui.sim_params = sim_params;
        self.lens_ui.needs_update = true;
        self.lens_ui.num_wavelengths /= 10;

        // save_png::save_png(
        //     &tex,
        //     size,
        //     &self.state.device,
        //     &self.state.queue,
        //     "test.png",
        // );

        let mut raw = self.raw.write().unwrap();

        *raw = Self::tex_to_raw(&tex, size, &self.state.device, &self.state.queue);

        // for r in raw.iter() {
        //     if (r.0 != 0 || r.1 != 0 || r.2 != 0) {
        //         println!("{:?}", r);
        //     }
        // }

        println!("Rendering and saving image took {:?}", now.elapsed());
    }
}
