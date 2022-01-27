#![allow(clippy::float_cmp)]
use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig, Texture, TextureConfig};
use itertools::iproduct;
use lens_state::LensState;
use polynomial_optics::{Polynom2d, Polynom4d};
use std::time::Instant;
use wgpu::Extent3d;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod texture;

mod state;
use state::State;

mod scenes;

mod lens_state;
mod save_png;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    /// whether to request lower requirements for the GPU
    #[structopt(short, long = "low_requirements")]
    low_req: bool,

    /// Set which api to use
    #[structopt(short, long, default_value = "all")]
    backend: String,

    #[structopt(short, long)]
    adapter: Option<usize>,

    #[structopt(short = "v", long)]
    disable_vsync: bool,
}

fn main() {
    let opt: Opt = Opt::from_args();

    println!("API: {:?}, low requirements: {}", opt.backend, opt.low_req);

    let backend = match opt.backend.to_lowercase().as_str() {
        "opengl" => wgpu::Backends::GL,
        "gl" => wgpu::Backends::GL,
        "vulkan" => wgpu::Backends::VULKAN,
        "dx" => wgpu::Backends::DX12,
        "dx12" => wgpu::Backends::DX12,
        "dx11" => wgpu::Backends::DX11,
        "metal" => wgpu::Backends::METAL,
        "all" => wgpu::Backends::all(),
        _ => panic!("unknown backend!"),
    };

    let event_loop = EventLoop::new();

    // state saves all the scenes and manages them
    let mut state = pollster::block_on(State::new(
        &event_loop,
        backend,
        opt.low_req,
        opt.adapter,
        opt.disable_vsync,
    ));
    let mut lens_ui: LensState = LensState::default(&state.device);

    // create scenes and push into state
    let mut poly_optics = pollster::block_on(scenes::PolyOptics::new(
        &state.device,
        &state.config,
        &lens_ui,
    ));

    let mut poly_res =
        pollster::block_on(scenes::PolyRes::new(&state.device, &state.config, &lens_ui));
    let mut poly_tri =
        pollster::block_on(scenes::PolyTri::new(&state.device, &state.config, &lens_ui));
    poly_optics.update_buffers(&state.device, true, &lens_ui);

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        &state.window,
        imgui_winit_support::HiDpiMode::Default,
    );
    // don't save imgui prefrences
    // imgui.set_ini_filename(None);

    // Set font for imgui
    {
        let hidpi_factor = state.window.scale_factor();
        poly_optics.resize(&state.device, &state.config, &lens_ui);
        // poly_res.resize(
        //     state.size,
        //     hidpi_factor,
        //     &state.device,
        //     &state.config,
        //     &state.queue,
        //     &lens_ui,
        // );
        println!("scaling factor: {}", hidpi_factor);

        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);
    }

    //
    // Set up dear imgui wgpu renderer
    //

    let mut renderer = {
        let renderer_config = RendererConfig {
            texture_format: state.config.format,
            ..Default::default()
        };
        Renderer::new(&mut imgui, &state.device, &state.queue, renderer_config)
    };

    let mut last_frame = Instant::now();

    let mut last_cursor = None;

    let mut poly_res_size: [f32; 2] = [640.0, 480.0];

    let res_window_texture_id = {
        let texture_config = TextureConfig {
            size: wgpu::Extent3d {
                width: poly_res_size[0] as u32,
                height: poly_res_size[1] as u32,
                ..Default::default()
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        };

        let texture = Texture::new(&state.device, &renderer, texture_config);
        renderer.textures.insert(texture)
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => state.window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                // handle input here if needed
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size, None);
                        poly_optics.resize(&state.device, &state.config, &lens_ui);
                        lens_ui.resize_main(state.size, state.scale_factor)
                    }
                    WindowEvent::ScaleFactorChanged {
                        new_inner_size,
                        scale_factor,
                        ..
                    } => {
                        state.resize(**new_inner_size, Some(scale_factor));
                        poly_optics.resize(&state.device, &state.config, &lens_ui);
                        poly_res.resize(
                            state.size,
                            *scale_factor * lens_ui.scale_fact,
                            &state.device,
                            &state.config,
                        );
                        poly_tri.resize(
                            state.size,
                            *scale_factor * lens_ui.scale_fact,
                            &state.device,
                            &state.config,
                        );
                    }
                    _ => {}
                }
            }
            Event::RedrawEventsCleared => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;

                // get entire window as texture
                let frame = match state.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("dropped frame: {:?}", e);
                        return;
                    }
                };
                platform
                    .prepare_frame(imgui.io_mut(), &state.window)
                    .expect("Failed to prepare frame");
                let ui = imgui.frame();

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let (update_lens, update_ray_num, update_dot_num, render, update_res, compute) =
                    lens_ui.build_ui(&ui, &state.device, &state.queue);

                // Render normally at background
                poly_optics.update(&state.device, &lens_ui);

                if lens_ui.triangulate {
                    poly_tri.update(&state.device, &lens_ui);
                } else {
                    poly_res.update(&state.device, &lens_ui);
                }

                if update_res {
                    poly_res.resize(
                        state.size,
                        state.scale_factor * lens_ui.scale_fact,
                        &state.device,
                        &state.config,
                    );

                    poly_tri.resize(
                        state.size,
                        state.scale_factor * lens_ui.scale_fact,
                        &state.device,
                        &state.config,
                    );
                }

                if update_dot_num {
                    poly_res.num_dots = 10.0_f64.powf(lens_ui.dots_exponent) as u32;
                    poly_tri.dot_side_len = 10.0_f64.powf(lens_ui.dots_exponent) as u32;
                    lens_ui.sim_params[11] = poly_tri.dot_side_len as f32;
                    lens_ui.needs_update = true;
                }
                // Dot/ray num
                if update_ray_num {
                    poly_optics.num_rays = 10.0_f64.powf(lens_ui.ray_exponent) as u32;
                }

                // Render debug view
                if update_lens | update_ray_num {
                    poly_optics.update_rays(&state.device, &state.queue, true, &lens_ui);
                }
                if lens_ui.draw_background {
                    match poly_optics.render(&view, &state.device, &state.queue, &lens_ui) {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size, None),
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    }
                }

                if render {
                    let now = Instant::now();
                    let size = [2048 * 4, 2048 * 4];
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
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_SRC,
                    };
                    let tex = state.device.create_texture(&desc);

                    if lens_ui.triangulate {
                        poly_tri.resize(
                            winit::dpi::PhysicalSize {
                                width: size[0],
                                height: size[1],
                            },
                            2.0,
                            &state.device,
                            &state.config,
                        );
                    } else {
                        poly_res.resize(
                            winit::dpi::PhysicalSize {
                                width: size[0],
                                height: size[1],
                            },
                            1.0,
                            &state.device,
                            &state.config,
                        );
                    }

                    let sim_params = lens_ui.sim_params;

                    lens_ui.resize_window(
                        winit::dpi::PhysicalSize {
                            width: size[0],
                            height: size[1],
                        },
                        1.0,
                    );

                    lens_ui.num_wavelengths *= 10;
                    lens_ui.update(&state.device, &state.queue);

                    if lens_ui.triangulate {
                        poly_tri
                            .render_hires(
                                &tex.create_view(&wgpu::TextureViewDescriptor::default()),
                                &state.device,
                                &state.queue,
                                &mut lens_ui,
                            )
                            .unwrap();

                        poly_tri.resize(
                            winit::dpi::PhysicalSize {
                                width: sim_params[9] as _,
                                height: sim_params[10] as _,
                            },
                            state.scale_factor,
                            &state.device,
                            &state.config,
                        );
                    } else {
                        poly_res
                            .render_hires(
                                &tex.create_view(&wgpu::TextureViewDescriptor::default()),
                                &state.device,
                                &state.queue,
                                10.0_f64.powf(lens_ui.hi_dots_exponent) as u64,
                                &mut lens_ui,
                            )
                            .unwrap();

                        poly_res.resize(
                            winit::dpi::PhysicalSize {
                                width: sim_params[9] as _,
                                height: sim_params[10] as _,
                            },
                            state.scale_factor,
                            &state.device,
                            &state.config,
                        );
                    }
                    lens_ui.sim_params = sim_params;
                    lens_ui.needs_update = true;
                    lens_ui.num_wavelengths /= 10;
                    save_png::save_png(&tex, size, &state.device, &state.queue, "hi-res.png");

                    println!("Rendering and saving image took {:?}", now.elapsed());
                }
                // poly_res.num_dots = u32::MAX / 32;//10.0_f64.powf(lens_ui.dots_exponent) as u32;

                if compute {
                    let old_pos_params = lens_ui.pos_params;
                    let old_sim_params = lens_ui.sim_params;
                    let which_ghost = lens_ui.which_ghost;
                    let size = [2048, 2048];
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
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_SRC,
                    };
                    let tex = state.device.create_texture(&desc);
                    poly_tri.resize(
                        winit::dpi::PhysicalSize {
                            width: size[0],
                            height: size[1],
                        },
                        2.0,
                        &state.device,
                        &state.config,
                    );
                    lens_ui.resize_window(
                        winit::dpi::PhysicalSize {
                            width: size[0],
                            height: size[1],
                        },
                        1.0,
                    );
                    lens_ui.update(&state.device, &state.queue);
                    poly_tri
                        .render(
                            &tex.create_view(&wgpu::TextureViewDescriptor::default()),
                            &state.device,
                            &state.queue,
                            &lens_ui,
                        )
                        .unwrap();
                    save_png::save_png(&tex, size, &state.device, &state.queue, "before.png");
                    let num_dots = 2;
                    let width = 2.;
                    let mut points = vec![];

                    for i in 0..num_dots {
                        for j in 0..num_dots {
                            lens_ui.pos_params[0] =
                                (i as f32 + 0.5) / (num_dots as f32) * width - width / 2.;
                            lens_ui.pos_params[1] =
                                (j as f32 + 0.5) / (num_dots as f32) * width - width / 2.;
                            println!("pos: {}, {}", lens_ui.pos_params[0], lens_ui.pos_params[1]);
                            lens_ui.update(&state.device, &state.queue);
                            // let dots =
                            //     poly_tri.get_dots(&state.device, &state.queue, true, &lens_ui);
                            let dots = lens_ui.actual_lens.get_dots(
                                poly_tri.dot_side_len * poly_tri.dot_side_len,
                                cgmath::Vector3 {
                                    x: lens_ui.pos_params[0] as f64,
                                    y: lens_ui.pos_params[1] as f64,
                                    z: lens_ui.pos_params[2] as f64,
                                },
                                cgmath::Vector3 {
                                    x: lens_ui.pos_params[4] as f64,
                                    y: lens_ui.pos_params[5] as f64,
                                    z: lens_ui.pos_params[6] as f64,
                                },
                                1,
                                lens_ui.pos_params[8] as f64,
                                false,
                            );
                            // println!(
                            //     "{}",
                            //     dots.iter()
                            //         //.filter(|point| point.strength.is_finite())
                            //         .map(|x| format!(
                            //             "x{} y{} s{} g{}",
                            //             x.init_pos[0], x.init_pos[1], x.strength, x.ghost_num
                            //         ))
                            //         .collect::<Vec<_>>()
                            //         .join("\n")
                            // );
                            // let ghost = dots;
                            // .chunks((poly_tri.dot_side_len * poly_tri.dot_side_len) as usize)
                            // .last()
                            // .unwrap();
                            for dot in dots.iter() {
                                let point = (
                                    dot.init_pos[0],
                                    dot.init_pos[1],
                                    dot.init_pos[2],
                                    dot.init_pos[3],
                                    // dot.strength,
                                    dot.pos[0],
                                );
                                points.push(point);
                            }
                        }
                    }
                    let points = points; // make points immutable
                    println!("points;: {}", points.len());
                    lens_ui.pos_params = old_pos_params;
                    lens_ui.update(&state.device, &state.queue);

                    let now = Instant::now();
                    let filtered_points = points
                        .clone()
                        .into_iter()
                        .filter(|point| point.4 > 0.0)
                        .collect::<Vec<_>>();
                    let polynom = Polynom4d::<_, 5>::fit(&filtered_points);
                    println!("Fitting took {:?}", now.elapsed());
                    println!("{}", polynom);
                    let sparse_poly = polynom.get_sparse(&filtered_points, 80);
                    println!("{}", sparse_poly);
                    let mut difference = 0.0;
                    let mut difference_sparse = 0.0;
                    for point in points.iter().filter(|point| point.4 > 0.0) {
                        let strength = polynom.eval(point.0, point.1, point.2, point.3);
                        difference += (strength - point.4).abs().powf(2.);
                        difference_sparse +=
                            (sparse_poly.eval([point.0, point.1, point.2, point.3]) - point.4)
                                .abs()
                                .powf(2.);
                        // println!(
                        //     "xyzw:{:?}: real: {}, {}",
                        //     (point.0, point.1, point.2, point.3),
                        //     point.4,
                        //     strength
                        // );
                    }
                    println!(
                        "average difference: {}",
                        (difference / points.len() as f32).sqrt()
                    );
                    println!(
                        "average difference sparse: {}",
                        (difference_sparse / points.len() as f32).sqrt()
                    );
                    // poly_tri.dot_side_len = 100;
                    // lens_ui.sim_params[11] = poly_tri.dot_side_len as f32;
                    // lens_ui.needs_update = true;
                    // lens_ui.update(&state.device, &state.queue);
                    // let dots = poly_tri.get_dots(&state.device, &state.queue, true, &lens_ui);
                    // let dots = lens_ui.actual_lens.get_dots(
                    //     poly_tri.dot_side_len * poly_tri.dot_side_len,
                    //     cgmath::Vector3 {
                    //         x: lens_ui.pos_params[0] as f64,
                    //         y: lens_ui.pos_params[1] as f64,
                    //         z: lens_ui.pos_params[2] as f64,
                    //     },
                    //     cgmath::Vector3 {
                    //         x: lens_ui.pos_params[4] as f64,
                    //         y: lens_ui.pos_params[5] as f64,
                    //         z: lens_ui.pos_params[6] as f64,
                    //     },
                    //     which_ghost,
                    //     lens_ui.pos_params[8] as f64,
                    //     false,
                    // );
                    // println!("dots: {:?}", dots[0..5].to_vec());
                    // let dots = lens_ui.actual_lens.get_dots(
                    //     100,
                    //     cgmath::Vector3 {
                    //         x: 0.,
                    //         y: 0.,
                    //         z: -6.,
                    //     },
                    //     cgmath::Vector3 {
                    //         x: 0.,
                    //         y: 0.,
                    //         z: 1.,
                    //     },
                    //     lens_ui.draw,
                    //     lens_ui.which_ghost,
                    //     lens_ui.pos_params[8].into(),
                    // );
                    // let draw_points = dots
                    //     .clone()
                    //     .into_iter()
                    //     // .map(|mut point| {
                    //     //     point.strength = polynom.eval(
                    //     //         lens_ui.pos_params[0],
                    //     //         lens_ui.pos_params[1],
                    //     //         point.init_pos[0],
                    //     //         point.init_pos[1],
                    //     //     );
                    //     //     point
                    //     // })
                    //     // .filter(|point| point.strength > 0.0 && point.ghost_num == 5)
                    //     .map(|mut point| {
                    //         let strength = point.strength;
                    //         // point.strength = sparse_poly.eval([
                    //         //     point.init_pos[0],
                    //         //     point.init_pos[1],
                    //         // ]);
                    //         // point.strength = polynom.eval(
                    //         //     lens_ui.pos_params[0],
                    //         //     lens_ui.pos_params[1],
                    //         //     point.init_pos[0],
                    //         //     point.init_pos[1],
                    //         // );
                    //         // println!(
                    //         //     "strength at {:?}: {} was {}",
                    //         //     (
                    //         //         lens_ui.pos_params[0],
                    //         //         lens_ui.pos_params[1],
                    //         //         point.init_pos[0],
                    //         //         point.init_pos[1],
                    //         //     ),
                    //         //     point.strength,
                    //         //     strength
                    //         // );
                    //         point
                    //     })
                    //     .into_iter()
                    //     .flat_map(|point| {
                    //         [
                    //             point.pos[0],
                    //             point.pos[1],
                    //             point.aperture_pos[0],
                    //             point.aperture_pos[1],
                    //             point.entry_pos[0],
                    //             point.entry_pos[1],
                    //             point.strength,
                    //             point.wavelength,
                    //         ]
                    //     })
                    //     .collect::<Vec<f32>>();
                    // println!("num points: {}", draw_points.len() / 8);
                    // state.queue.write_buffer(
                    //     &poly_tri.vertex_buffer,
                    //     0,
                    //     bytemuck::cast_slice(&draw_points),
                    // );

                    let grid_size = 100;
                    // let grid_string = polynom
                    //     .eval_grid(
                    //         &(0..grid_size)
                    //             .map(|x| (x - grid_size / 2) as f32 * 1. / grid_size as f32)
                    //             .collect::<Vec<f32>>(),
                    //         &(0..grid_size)
                    //             .map(|y| (y - grid_size / 2) as f32 * 1. / grid_size as f32)
                    //             .collect::<Vec<f32>>(),
                    //     )
                    //     .iter()
                    //     .map(|vec| {
                    //         vec.iter()
                    //             .map(|str| str.to_string())
                    //             .collect::<Vec<String>>()
                    //             .join(" ")
                    //     })
                    //     .collect::<Vec<String>>()
                    //     .join("\n");

                    println!("zero zero: {}", sparse_poly.eval([0., 0., 0., 0.]));

                    // let string = iproduct![(0..grid_size), (0..grid_size)]
                    //     .map(|(x, y)| {
                    //         [
                    //             ((x - grid_size / 2) as f32 * width / grid_size as f32) * width,
                    //             ((y - grid_size / 2) as f32 * width / grid_size as f32) * width,
                    //             polynom.eval(
                    //                 0.,
                    //                 0.,
                    //                 ((x - grid_size / 2) as f32 * width / grid_size as f32) * width,
                    //                 ((y - grid_size / 2) as f32 * width / grid_size as f32) * width,
                    //             ),
                    //             sparse_poly.eval([
                    //                 0.,
                    //                 0.,
                    //                 ((x - grid_size / 2) as f32 * width / grid_size as f32) * width,
                    //                 ((y - grid_size / 2) as f32 * width / grid_size as f32) * width,
                    //             ]),
                    //         ]
                    //     })
                    //     .map(|str| str.map(|str| str.to_string()).join(" "))
                    //     .collect::<Vec<String>>()
                    //     .join("\n");

                    let string = points
                        .iter()
                        .map(|point| {
                            //[point.2, point.3, point.4]
                            if point.0 == 0.5 && point.1 == 0.5 && point.4.is_finite() {
                                [
                                    point.2,
                                    point.3,
                                    point.4,
                                    polynom.eval(0., 0., point.2, point.3),
                                    sparse_poly.eval([0., 0., point.2, point.3]),
                                ]
                            } else {
                                [point.2, point.3, f32::NAN, f32::NAN, f32::NAN]
                            }
                            .map(|str| str.to_string())
                            .join(" ")
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    std::fs::write("plots/points.txt", string).unwrap();

                    // let string = dots
                    //     .iter()
                    //     .filter(|point| point.strength > 0.0 && point.ghost_num == 5)
                    //     .map(|point| {
                    //         [point.init_pos[0], point.init_pos[1], point.strength]
                    //             .map(|str| str.to_string())
                    //             .join(" ")
                    //     })
                    //     .collect::<Vec<String>>()
                    //     .join("\n");

                    let string = points
                        .iter()
                        .map(|point| {
                            //[point.2, point.3, point.4]
                            [point.2, point.3, point.4]
                                .map(|str| str.to_string())
                                .join(" ")
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    std::fs::write("plots/dots.txt", string).unwrap();

                    poly_tri
                        .render(
                            &tex.create_view(&wgpu::TextureViewDescriptor::default()),
                            &state.device,
                            &state.queue,
                            &lens_ui,
                        )
                        .unwrap();
                    save_png::save_png(&tex, size, &state.device, &state.queue, "after.png");
                    poly_tri.resize(
                        winit::dpi::PhysicalSize {
                            width: old_sim_params[9] as _,
                            height: old_sim_params[10] as _,
                        },
                        state.scale_factor,
                        &state.device,
                        &state.config,
                    );
                    lens_ui.sim_params = old_sim_params;
                }

                if lens_ui.triangulate {
                    // let mut encoder =
                    //     state
                    //         .device
                    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    //             label: Some("Render Encoder"),
                    //         });
                    // poly_tri.update_dots(&state.device, &mut encoder, , &lens_ui);
                    // state.queue.submit(Some(encoder.finish()));
                } else {
                    poly_res.update_dots(&state.device, &state.queue, update_dot_num, &lens_ui);
                }

                // Render PolyRes
                {
                    // Store the new size of Image() or None to indicate that the window is collapsed.
                    let mut new_window_size: Option<[f32; 2]> = None;
                    imgui::Window::new("Poly Res")
                        .size([512.0, 512.0], Condition::FirstUseEver)
                        .position([700., 50.], Condition::FirstUseEver)
                        .build(&ui, || {
                            new_window_size = Some(ui.content_region_avail());
                            imgui::Image::new(res_window_texture_id, new_window_size.unwrap())
                                .build(&ui);
                        });

                    if let Some(size) = new_window_size {
                        // Resize render target if size changed
                        if size != poly_res_size && size[0] >= 1.0 && size[1] >= 1.0 {
                            poly_res_size = size;
                            let scale = &ui.io().display_framebuffer_scale;
                            let texture_config = TextureConfig {
                                size: Extent3d {
                                    width: (poly_res_size[0] * scale[0]) as u32,
                                    height: (poly_res_size[1] * scale[1]) as u32,
                                    ..Default::default()
                                },
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                    | wgpu::TextureUsages::TEXTURE_BINDING,
                                ..Default::default()
                            };
                            renderer.textures.replace(
                                res_window_texture_id,
                                Texture::new(&state.device, &renderer, texture_config),
                            );

                            poly_res.resize(
                                size.into(),
                                state.scale_factor * lens_ui.scale_fact,
                                &state.device,
                                &state.config,
                            );

                            poly_tri.resize(
                                size.into(),
                                state.scale_factor * lens_ui.scale_fact,
                                &state.device,
                                &state.config,
                            );

                            lens_ui.resize_window(size.into(), state.scale_factor);
                        }

                        if lens_ui.triangulate {
                            match poly_tri.render_color(
                                renderer.textures.get(res_window_texture_id).unwrap().view(),
                                &state.device,
                                &state.queue,
                                &mut lens_ui,
                                update_dot_num | update_lens,
                            ) {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => {
                                    state.resize(state.size, None);
                                }
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    *control_flow = ControlFlow::Exit
                                }
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => eprintln!("{:?}", e),
                            }
                        } else {
                            // Only render contents if the window is not collapsed
                            match poly_res.render(
                                renderer.textures.get(res_window_texture_id).unwrap().view(),
                                &state.device,
                                &state.queue,
                                &lens_ui,
                            ) {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => {
                                    state.resize(state.size, None);
                                }
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    *control_flow = ControlFlow::Exit
                                }
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                    }
                }

                // Actually render imgui ui
                let mut encoder: wgpu::CommandEncoder = state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    if last_cursor != Some(ui.mouse_cursor()) {
                        last_cursor = Some(ui.mouse_cursor());
                        platform.prepare_render(&ui, &state.window);
                    }

                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Do not draw over debug
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    renderer
                        .render(ui.render(), &state.queue, &state.device, &mut rpass)
                        .expect("Rendering imgui failed");
                }
                state.queue.submit(Some(encoder.finish()));

                frame.present();
            }
            _ => {}
        }
        // Let imgui handle the event
        platform.handle_event(imgui.io_mut(), &state.window, &event);
    });
}
