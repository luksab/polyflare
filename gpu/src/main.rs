use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig, Texture, TextureConfig};
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{Instant, SystemTime};
use wgpu::Extent3d;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod camera;
mod instance;
mod model;
mod texture;

mod scene;
mod state;
use state::State;

mod scenes;

struct Parms {
    ray_exponent: f64,
    draw: u32,
    pos: [f64; 3],
    dir: [f64; 3],
    opacity: f32,
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();

    // initialize a state
    let mut state = pollster::block_on(State::new(&event_loop, wgpu::Backends::all()));

    let mut optics_params = Parms {
        ray_exponent: 2.7,
        draw: 3,
        pos: [0.0, 0.0, -5.0],
        dir: [0.0, 0.1, 1.0],
        opacity: 0.1,
    };

    // create scenes and push into state

    let game_of_life = Rc::new(Mutex::new(pollster::block_on(scenes::GameOfLife::new(
        &state.device,
        &state.config,
    ))));
    state.scenes.push(game_of_life.clone());

    let poly_optics = Rc::new(Mutex::new(pollster::block_on(scenes::PolyOptics::new(
        &state.device,
        &state.config,
    ))));
    state.scenes.push(poly_optics.clone());

    // let demo = pollster::block_on(scenes::Demo3d::new(
    //     &state.device,
    //     &state.queue,
    //     &state.config,
    // ));

    // state.scenes.push(Box::new(demo));

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
    imgui.set_ini_filename(None);

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

    let mut window_render_size: [f32; 2] = [640.0, 480.0];

    // Stores a texture for displaying with imgui::Image(),
    // also as a texture view for rendering into it
    let window_texture_id = {
        let texture_config = TextureConfig {
            size: wgpu::Extent3d {
                width: window_render_size[0] as u32,
                height: window_render_size[1] as u32,
                ..Default::default()
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        };

        let texture = Texture::new(&state.device, &renderer, texture_config);
        renderer.textures.insert(texture)
    };

    let depth_texture_id = {
        let depth_texture = Texture::new(
            &state.device,
            &renderer,
            TextureConfig {
                size: wgpu::Extent3d {
                    width: window_render_size[0] as u32,
                    height: window_render_size[1] as u32,
                    ..Default::default()
                },
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                format: Some(wgpu::TextureFormat::Depth32Float),
                ..Default::default()
            },
        );
        renderer.textures.insert(depth_texture)
    };

    event_loop.run(move |event, _, control_flow| {
        if last_sec.elapsed().unwrap().as_secs() > 1 {
            last_sec = SystemTime::now();
            fps = frames_since_last_sec;
            // println!("fps: {}", frames_since_last_sec);
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
                match state.render(&view, Some(&depth_texture.view), 1) {
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

                let mut poly = poly_optics.lock().unwrap();
                let mut update_poly = false;
                imgui::Window::new("Params")
                    .size([400.0, 250.0], Condition::FirstUseEver)
                    .position([600.0, 100.0], Condition::FirstUseEver)
                    .build(&ui, || {
                        ui.text(format!("Framerate: {:?}", fps));
                        update_poly |= Slider::new("rays_exponent", 0., 5.)
                            .build(&ui, &mut optics_params.ray_exponent);
                        ui.text(format!(
                            "rays: {}",
                            10.0_f64.powf(optics_params.ray_exponent) as u32
                        ));

                        if Slider::new("opacity", 0., 1.).build(&ui, &mut optics_params.opacity) {
                            poly.sim_params[0] = optics_params.opacity.powf(3.);
                            poly.write_buffer(&state.queue);
                        }

                        update_poly |= ui.radio_button("render nothing", &mut optics_params.draw, 0)
                            || ui.radio_button("render both", &mut optics_params.draw, 3)
                            || ui.radio_button("render normal", &mut optics_params.draw, 2)
                            || ui.radio_button("render ghosts", &mut optics_params.draw, 1);
                        poly.draw_mode = optics_params.draw;
                        // ui.radio_button("num_rays", &mut optics_params.1, true);
                        update_poly |= Drag::new("ray origin")
                            .speed(0.01)
                            .range(-10., 10.)
                            .build_array(&ui, &mut optics_params.pos);

                        update_poly |= Drag::new("ray direction")
                            .speed(0.01)
                            .range(-1., 1.)
                            .build_array(&ui, &mut optics_params.dir);
                    });

                poly.num_rays = 10.0_f64.powf(optics_params.ray_exponent) as u32;
                poly.center_pos = optics_params.pos.into();
                poly.direction = optics_params.dir.into();

                if update_poly {
                    poly.update_rays(&state.device);
                }

                // Store the new size of Image() or None to indicate that the window is collapsed.
                let mut new_window_size: Option<[f32; 2]> = None;
                imgui::Window::new("Hello World")
                    .size([512.0, 512.0], Condition::FirstUseEver)
                    .collapsed(true, Condition::FirstUseEver)
                    .build(&ui, || {
                        // new_example_size = Some(ui.content_region_avail());
                        // ui.text("Hello world!");
                        // ui.text("This...is...imgui-rs on WGPU!");
                        // ui.separator();
                        // let mouse_pos = ui.io().mouse_pos;
                        // ui.text(format!(
                        //     "Mouse Position: ({:.1},{:.1})",
                        //     mouse_pos[0], mouse_pos[1]
                        // ));
                        new_window_size = Some(ui.content_region_avail());
                        imgui::Image::new(window_texture_id, new_window_size.unwrap()).build(&ui);
                    });

                if let Some(size) = new_window_size {
                    // Resize render target, which is optional
                    if size != window_render_size && size[0] >= 1.0 && size[1] >= 1.0 {
                        window_render_size = size;
                        let scale = &ui.io().display_framebuffer_scale;
                        let texture_config = TextureConfig {
                            size: Extent3d {
                                width: (window_render_size[0] * scale[0]) as u32,
                                height: (window_render_size[1] * scale[1]) as u32,
                                ..Default::default()
                            },
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::TEXTURE_BINDING,
                            ..Default::default()
                        };
                        renderer.textures.replace(
                            window_texture_id,
                            Texture::new(&state.device, &renderer, texture_config),
                        );
                        let depth_texture = Texture::new(
                            &state.device,
                            &renderer,
                            TextureConfig {
                                size: wgpu::Extent3d {
                                    width: (window_render_size[0] * scale[0]) as u32,
                                    height: (window_render_size[1] * scale[1]) as u32,
                                    ..Default::default()
                                },
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                    | wgpu::TextureUsages::TEXTURE_BINDING,
                                format: Some(wgpu::TextureFormat::Depth32Float),
                                ..Default::default()
                            },
                        );
                        renderer.textures.replace(depth_texture_id, depth_texture);
                    }

                    // Only render contents if the window is not collapsed
                    match state.render(
                        &renderer.textures.get(window_texture_id).unwrap().view(),
                        Some(&renderer.textures.get(depth_texture_id).unwrap().view()),
                        0,
                    ) {
                        Ok(_) => {
                            frames_since_last_sec += 1;
                        }
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => {
                            state.resize(state.size, None);
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
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
                                load: wgpu::LoadOp::Load, // Do not clear
                                // load: wgpu::LoadOp::Clear(clear_color),
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
