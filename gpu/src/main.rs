use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig, Texture, TextureConfig};
use polynomial_optics::{Element, Glass, Properties};
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{Instant, SystemTime};
use wgpu::Extent3d;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod texture;

mod scene;
mod state;
use state::State;

mod scenes;

mod lens_state;

fn get_lens(lens_ui: &Vec<(f32, f32, f32, f32, bool)>) -> Vec<polynomial_optics::Element> {
    let mut elements: Vec<Element> = vec![];
    let mut dst: f32 = -5.;
    for element in lens_ui {
        dst += element.1;
        if element.4 {
            elements.push(Element {
                radius: element.0 as f64,
                properties: Properties::Glass(Glass {
                    ior: 1.5,
                    coating: (),
                    entry: true,
                    spherical: true,
                }),
                position: dst as f64,
            });
            dst += element.3;
            elements.push(Element {
                radius: element.2 as f64,
                properties: Properties::Glass(Glass {
                    ior: 1.5,
                    coating: (),
                    entry: false,
                    spherical: true,
                }),
                position: dst as f64,
            });
        } else {
            elements.push(Element {
                radius: element.0 as f64,
                properties: Properties::Aperture((element.2 * 10.) as u32),
                position: dst as f64,
            });
        }
    }
    elements
}
/// Parameter for the GUI
struct Parms {
    ray_exponent: f64,
    dots_exponent: f64,
    draw: u32,
    pos: [f32; 3],
    dir: [f32; 3],
    opacity: f32,
    sensor_dist: f32,
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();

    // state saves all the scenes and manages them
    let mut state = pollster::block_on(State::new(&event_loop, wgpu::Backends::all()));

    // create scenes and push into state
    let poly_optics = Rc::new(Mutex::new(pollster::block_on(scenes::PolyOptics::new(
        &state.device,
        &state.config,
    ))));
    state.scenes.push(poly_optics.clone());

    let poly_res = Rc::new(Mutex::new(pollster::block_on(scenes::PolyRes::new(
        &state.device,
        &state.config,
        poly_optics.clone(),
    ))));
    state.scenes.push(poly_res.clone());

    // r1, d1, r2, distance to next lens, is_glass
    let mut lens_ui: lens_state::LensState = Default::default();
    // init lens
    {
        let mut poly = poly_optics.lock().unwrap();
        poly.lens.elements = lens_ui.get_lens();
        poly.update_buffers(&state.queue, &state.device, true);
    }

    let mut last_render_time = std::time::Instant::now();
    let mut last_sec = SystemTime::now();
    let mut frames_since_last_sec = 0;
    let mut fps = 0;

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

    let mut first_frame = true;

    event_loop.run(move |event, _, control_flow| {
        if last_sec.elapsed().unwrap().as_secs() > 1 {
            last_sec = SystemTime::now();
            fps = frames_since_last_sec;
            frames_since_last_sec = 0;
        }

        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => state.window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                state.input(event);
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
                    }
                    WindowEvent::ScaleFactorChanged {
                        new_inner_size,
                        scale_factor,
                        ..
                    } => {
                        state.resize(**new_inner_size, Some(scale_factor));
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
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt);
                let depth_texture = texture::Texture::create_depth_texture(
                    &state.device,
                    &wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format: state.surface.get_preferred_format(&state.adapter).unwrap(),
                        width: state.size.width,
                        height: state.size.height,
                        present_mode: wgpu::PresentMode::Fifo,
                    },
                    "depth_texture",
                );

                // Render demo3d
                match state.render(&view, Some(&depth_texture.view), 0) {
                    Ok(_) => {
                        frames_since_last_sec += 1;
                    }
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size, None),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }

                let (update_lens, update_sensor, update_size, mut update_ray_num) =
                    lens_ui.build_ui(&ui);

                update_ray_num |= first_frame;

                let mut poly = poly_optics.lock().unwrap();
                let mut poly_res = poly_res.lock().unwrap();

                poly.num_rays = 10.0_f64.powf(lens_ui.ray_exponent) as u32;
                // poly_res.num_dots = u32::MAX / 32;//10.0_f64.powf(lens_ui.dots_exponent) as u32;
                poly_res.num_dots = 10.0_f64.powf(lens_ui.dots_exponent) as u32;
                poly_res.pos_params[0..3].copy_from_slice(&lens_ui.pos[0..3]);
                poly_res.pos_params[4..7].copy_from_slice(&lens_ui.dir[0..3]);

                poly.pos_params[0..3].copy_from_slice(&lens_ui.pos[0..3]);
                poly.pos_params[4..7].copy_from_slice(&lens_ui.dir[0..3]);

                poly.write_buffer(&state.queue);

                poly.update_rays(&state.device, &state.queue, update_ray_num);
                poly_res.update_rays(&poly, &state.device, &state.queue, update_ray_num);

                if update_sensor {
                    poly_res.pos_params[8] = lens_ui.sensor_dist;
                    poly_res.write_buffer(&state.queue);

                    poly.pos_params[8] = lens_ui.sensor_dist;
                    poly.write_buffer(&state.queue);
                }

                {
                    poly.sim_params[0] = lens_ui.opacity.powf(3.);
                    poly_res.sim_params[0] = lens_ui.opacity.powf(3.);
                    poly.write_buffer(&state.queue);
                    poly_res.write_buffer(&state.queue);
                }
                poly.draw_mode = lens_ui.draw;

                if update_lens {
                    poly.lens.elements = lens_ui.get_lens();
                    poly.update_buffers(&state.queue, &state.device, update_ray_num);
                    // we don't need poly for the rest of this function
                    drop(poly);
                    poly_res.update_buffers(&state.queue, &state.device, update_ray_num)
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

                            poly_res.sim_params[1] = size[0];
                            poly_res.sim_params[2] = size[1];
                            poly_res.sim_params[3] = size[0] * state.scale_factor as f32;
                            poly_res.sim_params[4] = size[1] * state.scale_factor as f32;
                            poly_res.write_buffer(&state.queue);
                        }
                        drop(poly_res);

                        // Only render contents if the window is not collapsed
                        match state.render(
                            &renderer.textures.get(res_window_texture_id).unwrap().view(),
                            None,
                            1,
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
                first_frame = false;
            }
            _ => {}
        }
        // Let imgui handle the event
        platform.handle_event(imgui.io_mut(), &state.window, &event);
    });
}
