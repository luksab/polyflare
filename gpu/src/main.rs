use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig, Texture, TextureConfig};
use lens_state::LensState;
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

fn main() {
    let event_loop = EventLoop::new();

    // state saves all the scenes and manages them
    let mut state = pollster::block_on(State::new(&event_loop, wgpu::Backends::all()));
    let mut lens_ui: LensState = LensState::default(&state.device);

    // create scenes and push into state
    let mut poly_optics = pollster::block_on(scenes::PolyOptics::new(
        &state.device,
        &state.config,
        &lens_ui,
    ));

    let mut poly_res =
        pollster::block_on(scenes::PolyRes::new(&state.device, &state.config, &lens_ui));
    poly_optics.update_buffers(&state.queue, &state.device, true, &lens_ui);

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
        poly_optics.resize(
            state.size,
            hidpi_factor,
            &state.device,
            &state.config,
            &state.queue,
            &lens_ui,
        );
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
                        poly_optics.resize(
                            state.size,
                            state.scale_factor,
                            &state.device,
                            &state.config,
                            &state.queue,
                            &lens_ui,
                        );
                    }
                    WindowEvent::ScaleFactorChanged {
                        new_inner_size,
                        scale_factor,
                        ..
                    } => {
                        state.resize(**new_inner_size, Some(scale_factor));
                        poly_optics.resize(
                            state.size,
                            *scale_factor,
                            &state.device,
                            &state.config,
                            &state.queue,
                            &lens_ui,
                        );
                        poly_res.resize(
                            state.size,
                            *scale_factor,
                            &state.device,
                            &state.config,
                            &state.queue,
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

                // Render normally at background
                poly_optics.update(&state.device, &lens_ui);
                poly_res.update(&state.device, &lens_ui);

                // Render debug view
                match poly_optics.render(&view, &state.device, &state.queue, &lens_ui) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size, None),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }

                let (update_lens, _update_size, update_ray_num, update_dot_num, render) =
                    lens_ui.build_ui(&ui, &state.device, &state.queue);

                if render {
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

                    poly_res.resize(
                        winit::dpi::PhysicalSize {
                            width: size[0],
                            height: size[0],
                        },
                        1.0,
                        &state.device,
                        &state.config,
                        &state.queue,
                    );

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
                        state.size,
                        state.scale_factor,
                        &state.device,
                        &state.config,
                        &state.queue,
                    );
                    save_png::save_png(&tex, size, &state.device, &state.queue);
                }
                // Dot/ray num
                poly_optics.num_rays = 10.0_f64.powf(lens_ui.ray_exponent) as u32;
                // poly_res.num_dots = u32::MAX / 32;//10.0_f64.powf(lens_ui.dots_exponent) as u32;
                poly_res.num_dots = 10.0_f64.powf(lens_ui.dots_exponent) as u32;

                poly_optics.draw_mode = lens_ui.draw;

                if update_lens {
                    poly_res.sim_params[5] = poly_optics.draw_mode as f32;
                    poly_optics.sim_params[5] = poly_optics.draw_mode as f32;
                    poly_res.sim_params[6] = lens_ui.which_ghost as f32;
                    poly_optics.sim_params[6] = lens_ui.which_ghost as f32;

                    poly_optics.sim_params[0] = lens_ui.opacity.powf(3.);
                    poly_res.sim_params[0] = lens_ui.opacity.powf(3.);
                    poly_optics.write_buffer(&state.queue);
                    poly_res.write_buffer(&state.queue);
                    // poly_optics.lens.elements = lens_ui.get_lens();
                    // poly_optics.update_buffers(&state.queue, &state.device, update_ray_num);
                    // poly_res.update_buffers(
                    //     &state.queue,
                    //     &state.device,
                    //     update_dot_num,
                    //     &poly_optics.lens_rt_data,
                    //     &poly_optics.lens_rt_buffer,
                    // )
                }

                poly_optics.update_rays(&state.device, &state.queue, update_ray_num, &lens_ui);
                poly_res.update_dots(&state.device, &state.queue, update_dot_num, &lens_ui);

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
                                state.scale_factor,
                                &state.device,
                                &state.config,
                                &state.queue,
                            );

                            poly_res.sim_params[1] = size[0];
                            poly_res.sim_params[2] = size[1];
                            poly_res.sim_params[3] = size[0] * state.scale_factor as f32;
                            poly_res.sim_params[4] = size[1] * state.scale_factor as f32;
                            poly_res.write_buffer(&state.queue);
                        }

                        // Only render contents if the window is not collapsed
                        match poly_res.render(
                            &renderer.textures.get(res_window_texture_id).unwrap().view(),
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
