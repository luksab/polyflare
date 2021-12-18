use std::fs::{self, DirBuilder};
use std::path::Path;
use std::time::Instant;

use cgmath::{InnerSpace, Vector3};
use directories::ProjectDirs;
use imgui::{Condition, Drag, Slider, Ui};
use polynomial_optics::{Element, Glass, Lens, Properties, Sellmeier};
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue};

mod sensor;

use sensor::*;

/// The representation of a piece of glass in the GUI
pub struct GlassElement {
    /// distance in front of the GlassElement
    d1: f32,
    /// radius of the front of the GlassElement
    r1: f32,
    /// distance from the middle of the front to the middle of the back side of the GlassElement
    d2: f32,
    /// radius of the back of the GlassElement
    r2: f32,
    /// whether this element is spherical or cylindrical
    spherical: bool,
    sellmeier: Sellmeier,
    /// index into `LensState.all_glasses`
    sellmeier_index: usize,
}

/// The representation of an aperture in the GUI
pub struct Aperture {
    /// distance in front of the Apterture
    d: f32,
    /// radius of the Apterture
    r: f32,
    /// number of blades of the Apterture
    num_blades: u32,
}

/// One Part of a Lens in the GUI
pub enum ElementState {
    Lens(GlassElement),
    Aperture(Aperture),
}

/// The state of the application
pub struct LensState {
    /// number of rays = 2^ray_exponent
    pub ray_exponent: f64,
    /// number of dots = 2^dots_exponent
    pub dots_exponent: f64,
    /// number of dots for high-res render = 2^dots_exponent
    pub hi_dots_exponent: f64,
    /// "render nothing": 0
    /// 
    /// "render both": 3
    /// 
    /// "render normal": 2
    /// 
    /// "render ghosts": 1
    pub draw: u32,
    /// multiplier for alpha of rays/dots
    pub opacity: f32,
    /// which ghost to draw: 0 being all, 1 being the fist...
    pub which_ghost: u32,

    /// GUI representation of the lens
    lens: Vec<ElementState>,
    /// The actual Lens being rendered
    pub actual_lens: Lens,
    /// index into all_lenses
    selected_lens: usize,
    /// filename of the currently selected lens
    current_filename: String,

    /// all types of glass
    all_glasses: Vec<(String, Sellmeier)>,

    /// index into all_sensors
    sensor_index: usize,
    /// all sensor representations
    all_sensors: Vec<(String, Sensor)>,
    /// buffer of the current sensor
    pub sensor_buffer: Buffer,
    /// positions of the rays and the sensor
    pub pos_params_buffer: wgpu::Buffer,
    /// positions of the rays and the sensor
    pub pos_bind_group: wgpu::BindGroup,
    /// positions of the rays and the sensor
    pub pos_bind_group_layout: wgpu::BindGroupLayout,
    /// Data for the positions of the rays and the sensor
    pub pos_params: [f32; 12],

    /// buffer for the currently selected lens
    lens_buffer: Buffer,
    /// buffer for the currently selected lens for raytracing
    lens_rt_buffer: Buffer,
    /// bind group for both representations of the current lens
    pub lens_bind_group: wgpu::BindGroup,
    /// bind group layout for both representations of the current lens
    pub lens_bind_group_layout: wgpu::BindGroupLayout,

    last_frame_time: Instant,
    fps: f64,

    /// are we rendering the first frame right now?
    first_frame: bool,
}

impl LensState {
    /// just some random (very bad) lens
    pub fn default(device: &Device) -> Self {
        let lens = vec![
            ElementState::Lens(GlassElement {
                d1: 0.,
                r1: 3.,
                d2: 1.5,
                r2: 3.,
                spherical: true,
                sellmeier: Sellmeier::bk7(),
                sellmeier_index: 0,
            }),
            ElementState::Aperture(Aperture {
                d: 1.5,
                r: 1.,
                num_blades: 6,
            }),
            ElementState::Lens(GlassElement {
                d1: 0.,
                r1: 3.,
                d2: 1.5,
                r2: 3.,
                spherical: true,
                sellmeier: Sellmeier::bk7(),
                sellmeier_index: 0,
            }),
        ];
        let sensor_dist = 3.;
        let actual_lens = Lens::new(Self::get_lens_arr(&lens), sensor_dist);

        let lens_rt_data = actual_lens.get_rt_elements_buffer();
        let lens_rt_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lens tracing Buffer"),
            contents: bytemuck::cast_slice(&lens_rt_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let lens_data = actual_lens.get_elements_buffer();
        let lens_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lens drawing Buffer"),
            contents: bytemuck::cast_slice(&lens_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let sensors = Sensor::get_all_sensors();
        let sensor_index = 0;
        let sensor_data = sensors[sensor_index].1.get_data();

        let sensor_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sensor info Buffer"),
            contents: bytemuck::cast_slice(&sensor_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let lens_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (lens_rt_data.len() * std::mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (lens_data.len() * std::mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let lens_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &lens_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lens_rt_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lens_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        let center_pos = Vector3 {
            x: 0.0,
            y: 0.0,
            z: -6.,
        };
        let direction = Vector3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
        .normalize();
        let pos_params = [
            center_pos.x as f32,
            center_pos.y as f32,
            center_pos.z as f32,
            0.,
            direction.x as f32,
            direction.y as f32,
            direction.z as f32,
            1.,
            7.,
            0.5, // Padding
            0.,  // Padding
            0.,  // Padding
        ];
        let pos_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Parameter Buffer"),
            contents: bytemuck::cast_slice(&pos_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let pos_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (pos_params.len() * std::mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sensor_data.len() * std::mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let pos_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &pos_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pos_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sensor_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        Self {
            ray_exponent: 5.,
            dots_exponent: 7.,
            hi_dots_exponent: 10.,
            draw: 1,
            opacity: 0.75,
            which_ghost: 0,
            lens,
            last_frame_time: Instant::now(),
            fps: 0.,
            actual_lens,
            selected_lens: 0,
            current_filename: String::new(),
            all_glasses: Sellmeier::get_all_glasses(),
            all_sensors: sensors,
            sensor_index,
            sensor_buffer,
            lens_rt_buffer,
            lens_buffer,
            pos_params_buffer,
            pos_bind_group,
            pos_params,
            pos_bind_group_layout,
            lens_bind_group,
            lens_bind_group_layout,
            first_frame: true,
        }
    }
}

impl LensState {
    /// Convert from the GUI representation to the `polynomial_optics` representation
    fn get_lens_arr(lens: &Vec<ElementState>) -> Vec<polynomial_optics::Element> {
        let mut elements: Vec<Element> = vec![];
        let mut dst: f32 = -5.;
        for element in lens {
            match element {
                ElementState::Lens(lens) => {
                    dst += lens.d1;
                    elements.push(Element {
                        radius: lens.r1 as f64,
                        properties: Properties::Glass(Glass {
                            sellmeier: lens.sellmeier,
                            coating: (),
                            entry: true,
                            spherical: lens.spherical,
                        }),
                        position: dst as f64,
                    });
                    dst += lens.d2;
                    elements.push(Element {
                        radius: lens.r2 as f64,
                        properties: Properties::Glass(Glass {
                            sellmeier: lens.sellmeier,
                            coating: (),
                            entry: false,
                            spherical: lens.spherical,
                        }),
                        position: dst as f64,
                    });
                }
                ElementState::Aperture(aperture) => {
                    dst += aperture.d;
                    elements.push(Element {
                        radius: aperture.r as f64,
                        properties: Properties::Aperture(aperture.num_blades),
                        position: dst as f64,
                    });
                }
            }
        }
        elements
    }
    /// get the `polynomial_optics` representation of the Lens
    pub fn get_lens(&self) -> Vec<polynomial_optics::Element> {
        Self::get_lens_arr(&self.lens)
    }

    /// Convert from the `polynomial_optics` representation to the GUI representation
    fn get_lens_state(&self) -> Vec<ElementState> {
        let mut elements = vec![];
        let mut last_pos = -5.;
        let mut expect_entry = true;
        let mut enty = (0., 0.);

        for element in &self.actual_lens.elements {
            match element.properties {
                Properties::Glass(glass) => {
                    if expect_entry && glass.entry {
                        enty = (element.position as f32 - last_pos, element.radius as f32);
                        expect_entry = false;
                    } else if !expect_entry && !glass.entry {
                        let mut sellmeier_index = 0;
                        for (index, (_name, other_glass)) in self.all_glasses.iter().enumerate() {
                            if glass.sellmeier == *other_glass {
                                sellmeier_index = index;
                            }
                        }

                        elements.push(ElementState::Lens(GlassElement {
                            d1: enty.0,
                            r1: enty.1,
                            d2: element.position as f32 - last_pos,
                            r2: element.radius as f32,
                            spherical: glass.spherical,
                            sellmeier: glass.sellmeier,
                            sellmeier_index,
                        }));
                        expect_entry = true;
                    } else {
                        panic!("expected entry != actual entry");
                    }
                }
                Properties::Aperture(num_blades) => {
                    elements.push(ElementState::Aperture(Aperture {
                        d: element.position as f32 - last_pos,
                        r: element.radius as f32,
                        num_blades,
                    }))
                }
            };
            last_pos = element.position as f32;
        }
        elements
    }

    /// update the buffers from the internal state
    pub fn update(&mut self, device: &Device, queue: &Queue) {
        self.actual_lens = Lens::new(self.get_lens(), self.actual_lens.sensor_dist);

        let lens_rt_data = self.actual_lens.get_rt_elements_buffer();
        self.lens_rt_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lens tracing Buffer"),
            contents: bytemuck::cast_slice(&lens_rt_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let lens_data = self.actual_lens.get_elements_buffer();
        self.lens_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lens drawing Buffer"),
            contents: bytemuck::cast_slice(&lens_data),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        self.lens_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.lens_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lens_rt_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.lens_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        self.pos_params[8] = self.actual_lens.sensor_dist;
        queue.write_buffer(
            &self.pos_params_buffer,
            0,
            bytemuck::cast_slice(&self.pos_params),
        );
    }

    /// read the available lens descriptions from ~/.config/polyflare/lenses/
    /// ```
    /// println!("{:?}", get_lenses());
    /// ```
    pub fn get_lenses() -> Vec<(String, Lens)> {
        let mut lenses = vec![];

        let proj_dirs = ProjectDirs::from("de", "luksab", "polyflare").unwrap();
        let dir = proj_dirs.config_dir().join(Path::new("lenses"));
        if dir.is_dir() {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();

                if !path.is_dir() {
                    match Lens::read(&path) {
                        Ok(lens) => lenses.push((
                            path.file_name().unwrap().to_owned().into_string().unwrap(),
                            lens,
                        )),
                        Err(str) => println!("Could not parse {:?}:\n\t {}", path, str),
                    }
                }
            }
        } else {
            println!("creating lens directory {:?}", dir);
            DirBuilder::new().recursive(true).create(dir).unwrap();
        }

        lenses
    }

    /// save the lens descriptions to ~/.config/polyflare/lenses/{name}
    pub fn save_lens(name: &str, lens: &Lens) {
        let proj_dirs = ProjectDirs::from("de", "luksab", "polyflare").unwrap();
        let dir = proj_dirs.config_dir().join(Path::new("lenses"));
        if !dir.is_dir() {
            println!("creating lens directory {:?}", &dir);
            DirBuilder::new().recursive(true).create(&dir).unwrap();
        }

        lens.save(&dir.join(Path::new(&name))).unwrap();
    }

    /// create an imgui window from Self and return
    ///
    /// (update_lens, update_lens_size, update_ray_num, update_dot_num, render)
    pub fn build_ui(
        &mut self,
        ui: &Ui,
        device: &Device,
        queue: &Queue,
    ) -> (bool, bool, bool, bool, bool) {
        let mut update_lens = self.first_frame;
        let mut update_sensor = self.first_frame;
        imgui::Window::new("Lens")
            .size([400.0, 250.0], Condition::FirstUseEver)
            .position([100.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                let mut lenses = Self::get_lenses();
                lenses.sort_by_key(|(name, _lens)| name.to_owned());

                if ui.combo(
                    "select lens",
                    &mut self.selected_lens,
                    lenses.as_slice(),
                    |(label, _lens)| std::borrow::Cow::Borrowed(label),
                ) {
                    self.actual_lens = lenses[self.selected_lens].1.clone();
                    self.lens = self.get_lens_state();
                    self.current_filename = lenses[self.selected_lens].0.clone();
                    update_lens = true;
                }

                if ui.combo(
                    "select sensor",
                    &mut self.sensor_index,
                    self.all_sensors.as_slice(),
                    |(label, _lens)| std::borrow::Cow::Borrowed(label),
                ) {
                    queue.write_buffer(
                        &self.sensor_buffer,
                        0,
                        bytemuck::cast_slice(&self.all_sensors[self.sensor_index].1.get_data()),
                    );
                }

                for (i, element) in self.lens.iter_mut().enumerate() {
                    match element {
                        ElementState::Lens(lens) => {
                            ui.push_item_width(ui.window_size()[0] / 2. - 45.);
                            ui.text(format!("Lens: {:?}", i + 1));

                            ui.same_line();
                            if ui.combo(
                                format!("select glass##{}", i),
                                &mut lens.sellmeier_index,
                                self.all_glasses.as_slice(),
                                |(label, _lens)| std::borrow::Cow::Borrowed(label),
                            ) {
                                lens.sellmeier = self.all_glasses[lens.sellmeier_index].1;
                                update_lens = true;
                            }
                            ui.same_line();
                            update_lens |= ui.checkbox(format!("spherical##{}", i), &mut lens.spherical);

                            update_lens |=
                                Slider::new(format!("d1##{}", i), 0., 5.).build(&ui, &mut lens.d1);
                            ui.same_line();
                            update_lens |=
                                Slider::new(format!("r1##{}", i), -6., 3.).build(&ui, &mut lens.r1);
                            update_lens |=
                                Slider::new(format!("d2##{}", i), -3., 6.).build(&ui, &mut lens.d2);
                            ui.same_line();
                            update_lens |=
                                Slider::new(format!("r2##{}", i), -6., 3.).build(&ui, &mut lens.r2);
                            ui.push_item_width(0.);

                            // update_size |= ui.checkbox(format!("button##{}", i), &mut element.4);
                            // update_lens |= update_size;
                            ui.separator();
                        }
                        ElementState::Aperture(aperture) => {
                            ui.text(format!("Aperture: {:?}", i + 1));
                            update_lens |= Slider::new(format!("d##{}", i), 0., 5.)
                                .build(&ui, &mut aperture.d);
                            update_lens |= Slider::new(format!("r1##{}", i), 0., 3.)
                                .build(&ui, &mut aperture.r);
                            update_lens |= Slider::new(format!("num_blades##{}", i), 3, 6)
                                .build(&ui, &mut aperture.num_blades);

                            // update_size |= ui.checkbox(format!("button##{}", i), &mut element.4);
                            // update_lens |= update_size;
                            ui.separator();
                        }
                    }
                }

                ui.same_line();
                if ui.button("save as") {
                    if self.current_filename.len() > 0 {
                        Self::save_lens(self.current_filename.as_str(), &self.actual_lens);
                    } else {
                        ui.open_popup("no name selected");
                    }
                }
                ui.same_line();
                ui.input_text("filename", &mut self.current_filename)
                    .build();
                ui.popup("no name selected", || {
                    ui.text("No name selected");
                    if ui.button("OK") {
                        ui.close_current_popup();
                    }
                });

                update_sensor |= Slider::new("sensor distance", 0., 20.)
                    .build(&ui, &mut self.actual_lens.sensor_dist);
                update_lens |= update_sensor
            });

        let mut update_rays = self.first_frame;
        let mut update_dots = self.first_frame;
        let mut render = false;

        let sample = 1. / (Instant::now() - self.last_frame_time).as_secs_f64();
        let alpha = 0.98;
        self.fps = alpha * self.fps + (1.0 - alpha) * sample;
        imgui::Window::new("Params")
            .size([400.0, 250.0], Condition::FirstUseEver)
            .position([600.0, 100.0], Condition::FirstUseEver)
            .build(&ui, || {
                let num_ghosts = (self.lens.len() * self.lens.len()) as u32;
                update_lens |=
                    Slider::new("which ghost", 0, num_ghosts + 1).build(&ui, &mut self.which_ghost);
                ui.text(format!("Framerate: {:.0}", self.fps));
                update_rays |=
                    Slider::new("rays_exponent", 0., 6.5).build(&ui, &mut self.ray_exponent);
                ui.text(format!("rays: {}", 10.0_f64.powf(self.ray_exponent) as u32));

                update_dots |=
                    Slider::new("dots_exponent", 0., 7.8).build(&ui, &mut self.dots_exponent);
                ui.text(format!(
                    "dots: {}",
                    10.0_f64.powf(self.dots_exponent) as u32
                ));

                update_lens |= Slider::new("opacity", 0., 4.).build(&ui, &mut self.opacity);

                update_lens |= ui.radio_button("render nothing", &mut self.draw, 0)
                    || ui.radio_button("render both", &mut self.draw, 3)
                    || ui.radio_button("render normal", &mut self.draw, 2)
                    || ui.radio_button("render ghosts", &mut self.draw, 1);

                // ui.radio_button("num_rays", &mut lens_ui.1, true);
                update_lens |= Drag::new("ray origin")
                    .speed(0.01)
                    .range(-10., 10.)
                    .build_array(&ui, &mut self.pos_params[0..3]);

                update_lens |= Drag::new("ray direction")
                    .speed(0.01)
                    .range(-1., 1.)
                    .build_array(&ui, &mut self.pos_params[4..7]);

                update_lens |= Slider::new("ray width", 0., 1.).build(&ui, &mut self.pos_params[9]);

                render = ui.button("hi-res render");
                ui.same_line();
                Slider::new("num_hi_rays", 0., 12.).build(&ui, &mut self.hi_dots_exponent);
            });

        if update_lens {
            self.update(device, queue);
        }

        self.last_frame_time = Instant::now();

        self.first_frame = false;
        (update_lens, false, update_rays, update_dots, render)
    }
}
