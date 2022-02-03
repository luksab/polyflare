use std::fs::{self, DirBuilder};
use std::path::Path;
use std::time::Instant;

use cgmath::{InnerSpace, Vector3};
use directories::ProjectDirs;
use imgui::{CollapsingHeader, Condition, Drag, Slider, Ui};
use polynomial_optics::{Element, Glass, Lens, Properties, QuarterWaveCoating, Sellmeier, Polynomial};
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue};

mod sensor;

use sensor::*;

/// The representation of a piece of glass in the GUI
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct GlassElement {
    /// distance in front of the GlassElement
    d: f32,
    /// radius of the front of the GlassElement
    r: f32,
    /// whether this element is the first or second part of a Lens
    entry: bool,
    /// whether this element is spherical or cylindrical
    spherical: bool,
    sellmeier: Sellmeier,
    /// index into `LensState.all_glasses`
    sellmeier_index: usize,
    coating_optimal: f32,
    coating_enable: bool,
}

/// The representation of an aperture in the GUI
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Aperture {
    /// distance in front of the Apterture
    d: f32,
    /// radius of the Apterture
    r: f32,
    /// number of blades of the Apterture
    num_blades: u32,
}

/// One Part of a Lens in the GUI
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum ElementState {
    Lens(GlassElement),
    Aperture(Aperture),
}

/// The state of the application
pub struct LensState {
    /// whether buffers need updating
    pub needs_update: bool,
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
    /// over or under-sample dot/tri texture
    pub scale_fact: f64,
    /// which ghost to draw: 0 being all, 1 being the fist...
    pub which_ghost: u32,
    /// number of wavelengths to render
    pub num_wavelengths: u32,
    /// whether to draw using triangulation or direct raytracing
    pub triangulate: bool,
    /// whether to draw the background
    pub draw_background: bool,

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
    /// buffer for which ghost to draw
    pub ghost_indices: Vec<[u32; 2]>,
    pub ghost_indices_buffer: wgpu::Buffer,
    /// positions of the rays and the sensor
    pub params_bind_group: wgpu::BindGroup,
    /// positions of the rays and the sensor
    pub params_bind_group_layout: wgpu::BindGroupLayout,
    /// Data for the positions of the rays and the sensor
    /// ```
    /// pos_params {
    /// 0: ox: f32;
    /// 1: oy: f32;
    /// 2: oz: f32;
    /// 3: wavelength: f32;
    /// 4: dx: f32;
    /// 5: dy: f32;
    /// 6: dz: f32;
    /// 7: strength: f32;
    /// 8: sensor: f32;
    /// 9: width: f32;
    /// padding
    /// };
    /// ```
    pub pos_params: [f32; 12],
    sim_param_buffer: wgpu::Buffer,
    /// ```
    /// struct SimParams {
    ///   0:opacity: f32;
    ///   1:width_scaled: f32;
    ///   2:height_scaled: f32;
    ///   3:width: f32;
    ///   4:height: f32;
    ///   5:draw_mode: f32;
    ///   6:which_ghost: f32;
    ///   7:window_width_scaled: f32;
    ///   8:window_height_scaled: f32;
    ///   9:window_width: f32;
    ///  10:window_height: f32;
    ///  11:side_len: f32;
    ///  12:zoom: f32;
    /// };
    /// ```
    pub sim_params: [f32; 13],

    /// buffer for the currently selected lens
    lens_buffer: Buffer,
    /// buffer for the currently selected lens for raytracing
    lens_rt_buffer: Buffer,
    /// bind group for both representations of the current lens
    pub lens_bind_group: wgpu::BindGroup,
    /// bind group layout for both representations of the current lens
    pub lens_bind_group_layout: wgpu::BindGroupLayout,

    // pub polys: Vec<Box< dyn PolyStore<f32>>>,
    /// buffer containing the sparse polynomial
    // pub poly_buffer: Buffer,

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
                d: 0.,
                r: 3.,
                entry: true,
                spherical: true,
                sellmeier: Sellmeier::bk7(),
                sellmeier_index: 0,
                coating_optimal: 0.5,
                coating_enable: false,
            }),
            ElementState::Lens(GlassElement {
                d: 1.5,
                r: 3.,
                entry: false,
                spherical: true,
                sellmeier: Sellmeier::bk7(),
                sellmeier_index: 0,
                coating_optimal: 0.5,
                coating_enable: false,
            }),
            ElementState::Aperture(Aperture {
                d: 1.5,
                r: 1.,
                num_blades: 6,
            }),
            ElementState::Lens(GlassElement {
                d: 0.,
                r: 3.,
                entry: true,
                spherical: true,
                sellmeier: Sellmeier::bk7(),
                sellmeier_index: 0,
                coating_optimal: 0.5,
                coating_enable: false,
            }),
            ElementState::Lens(GlassElement {
                d: 1.5,
                r: 3.,
                entry: false,
                spherical: true,
                sellmeier: Sellmeier::bk7(),
                sellmeier_index: 0,
                coating_optimal: 0.5,
                coating_enable: false,
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

        let ghost_indices = vec![[0, 1]];
        let ghost_indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ghost Indices Buffer"),
            contents: bytemuck::cast_slice(&ghost_indices),
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
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE
                            | wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ghost_indices_buffer.as_entire_binding(),
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
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        // buffer for simulation parameters uniform
        let sim_params = [
            0.1, 512., 512., 512., 512., 1.0, 1.0, 512., 512., 512., 512., 0., 4.,
        ];
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Parameter Buffer"),
            contents: bytemuck::cast_slice(&sim_params),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE
                            | wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT,
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
                        visibility: wgpu::ShaderStages::COMPUTE
                            | wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sensor_data.len() * std::mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE
                            | wgpu::ShaderStages::VERTEX
                            | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_params.len() * std::mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &params_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pos_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sensor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_param_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        Self {
            needs_update: false,
            ray_exponent: 5.,
            dots_exponent: 1.,
            hi_dots_exponent: 10.,
            draw: 1,
            triangulate: true,
            draw_background: true,
            opacity: 0.75,
            scale_fact: 1.,
            which_ghost: 0,
            num_wavelengths: 3,
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
            params_bind_group,
            pos_params,
            params_bind_group_layout,
            lens_bind_group,
            lens_bind_group_layout,
            ghost_indices,
            ghost_indices_buffer,
            first_frame: true,
            sim_param_buffer,
            sim_params,
        }
    }
}

impl LensState {
    pub fn resize_main(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: f64) {
        self.sim_params[1] = new_size.width as f32 * scale_factor as f32;
        self.sim_params[2] = new_size.height as f32 * scale_factor as f32;
        self.sim_params[3] = new_size.width as f32;
        self.sim_params[4] = new_size.height as f32;
        self.needs_update = true;
    }

    pub fn resize_window(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: f64) {
        self.sim_params[7] = new_size.width as f32 * scale_factor as f32;
        self.sim_params[8] = new_size.height as f32 * scale_factor as f32;
        self.sim_params[9] = new_size.width as f32;
        self.sim_params[10] = new_size.height as f32;
        self.needs_update = true;
    }

    /// Convert from the GUI representation to the `polynomial_optics` representation
    fn get_lens_arr(lenses: &[ElementState]) -> Vec<polynomial_optics::Element> {
        let mut elements: Vec<Element> = vec![];
        let mut dst: f32 = -5.;
        // let mut cemented = false;
        let mut entry = true;
        for (i, element) in lenses.iter().enumerate() {
            match element {
                ElementState::Lens(lens) => {
                    dst += lens.d;

                    let outer_ior = if lens.entry && lens.d == 0. && i > 0 {
                        if let ElementState::Lens(lens) = &lenses[i - 1] {
                            lens.sellmeier
                        } else {
                            Sellmeier::air()
                        }
                    } else if !lens.entry && i + 1 < lenses.len() {
                        match &lenses[i + 1] {
                            ElementState::Lens(lens) => {
                                if lens.d == 0. {
                                    lens.sellmeier
                                } else {
                                    Sellmeier::air()
                                }
                            }
                            ElementState::Aperture(_) => Sellmeier::air(),
                        }
                    } else {
                        Sellmeier::air()
                    };
                    elements.push(Element {
                        radius: lens.r as f64,
                        properties: Properties::Glass(Glass {
                            sellmeier: lens.sellmeier,
                            coating: if lens.coating_enable {
                                QuarterWaveCoating::optimal(
                                    lens.sellmeier.ior(lens.coating_optimal as f64),
                                    1.0,
                                    lens.coating_optimal as f64,
                                )
                            } else {
                                QuarterWaveCoating::none()
                            },
                            entry,
                            outer_ior,
                            spherical: lens.spherical,
                        }),
                        position: dst as f64,
                    });
                    entry = !entry;
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
    pub fn get_lens(&self) -> Lens {
        Lens::new(Self::get_lens_arr(&self.lens), self.actual_lens.sensor_dist)
    }

    /// Convert from the `polynomial_optics` representation to the GUI representation
    #[allow(dead_code)]
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

                        let coating_enable = glass.coating.thickness > 0.;
                        elements.push(ElementState::Lens(GlassElement {
                            d: enty.0,
                            r: enty.1,
                            entry: true,
                            spherical: glass.spherical,
                            sellmeier: glass.sellmeier,
                            sellmeier_index,
                            coating_optimal: 0.5, // TODO: read from file somehow
                            coating_enable,
                        }));
                        elements.push(ElementState::Lens(GlassElement {
                            d: element.position as f32 - last_pos,
                            r: element.radius as f32,
                            entry: false,
                            spherical: glass.spherical,
                            sellmeier: glass.sellmeier,
                            sellmeier_index,
                            coating_optimal: 0.5, // TODO: read from file somehow
                            coating_enable,
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
        self.sim_params[0] = self.opacity * self.opacity / self.num_wavelengths as f32;

        self.sim_params[5] = self.draw as f32;
        self.sim_params[6] = self.which_ghost as f32;

        self.actual_lens = self.get_lens();

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

        self.ghost_indices = self
            .actual_lens
            .get_ghosts_indicies(self.draw as _, self.which_ghost as _);

        self.ghost_indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ghost Indices Buffer"),
            contents: bytemuck::cast_slice(&self.ghost_indices),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.ghost_indices_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        self.pos_params[8] = self.actual_lens.sensor_dist as f32;
        queue.write_buffer(
            &self.pos_params_buffer,
            0,
            bytemuck::cast_slice(&self.pos_params),
        );
        queue.write_buffer(
            &self.sim_param_buffer,
            0,
            bytemuck::cast_slice(&self.sim_params),
        );
    }

    /// read the available lens descriptions from ~/.config/polyflare/lenses/
    /// ```
    /// println!("{:?}", get_lenses());
    /// ```
    pub fn get_lenses() -> Vec<(String, Vec<ElementState>)> {
        let mut lenses = vec![];

        let proj_dirs = ProjectDirs::from("de", "luksab", "polyflare").unwrap();
        let dir = proj_dirs.config_dir().join(Path::new("lenses"));
        if dir.is_dir() {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();

                if !path.is_dir() {
                    match Self::read_lens(&path) {
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

    /// read the lens descriptions from path
    fn read_lens(path: &Path) -> Result<Vec<ElementState>, String> {
        if let Ok(str) = std::fs::read_to_string(path) {
            return match ron::de::from_str(str.as_str()) {
                Ok(lens) => Ok(lens),
                Err(err) => Err(format!("{}", err)),
            };
        }
        Err(String::from("problem reading file"))
    }

    /// save the lens descriptions to ~/.config/polyflare/lenses/{name}
    fn save(&self, name: &str) -> std::io::Result<()> {
        let proj_dirs = ProjectDirs::from("de", "luksab", "polyflare").unwrap();
        let dir = proj_dirs.config_dir().join(Path::new("lenses"));
        if !dir.is_dir() {
            println!("creating lens directory {:?}", &dir);
            DirBuilder::new().recursive(true).create(&dir).unwrap();
        }

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&dir.join(Path::new(&name)))?;
        let pretty_config = ron::ser::PrettyConfig::new();
        std::io::Write::write_all(
            &mut file,
            ron::ser::to_string_pretty(&self.lens, pretty_config)
                .unwrap()
                .as_bytes(),
        )?;
        // handle errors
        file.sync_all()?;
        Ok(())
    }

    /// create an imgui window from Self and return
    ///
    /// (update_lens, update_lens_size, update_ray_num, update_dot_num, render, update_res, compute)
    pub fn build_ui(
        &mut self,
        ui: &Ui,
        device: &Device,
        queue: &Queue,
    ) -> (bool, bool, bool, bool, bool, bool) {
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
                    self.lens = lenses[self.selected_lens].1.clone();
                    self.actual_lens = self.get_lens();
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

                let mut delete_glass = None;
                let mut delete_aperture = None;

                let mut next_sellmeier = None;
                // how many elements to draw within the current header
                let mut ui_draw_next = 0;
                let mut element_counter = 0;
                let mut aperture_counter = 0;
                for (i, element) in self.lens.iter_mut().enumerate() {
                    match element {
                        ElementState::Lens(lens) => {
                            if lens.entry {
                                element_counter += 1;
                                if CollapsingHeader::new(format!("Element {:?}", element_counter))
                                    .build(ui)
                                {
                                    ui_draw_next += 2;
                                    ui.push_item_width(ui.window_size()[0] / 2. - 45.);

                                    if ui.combo(
                                        format!("select glass##{}", i),
                                        &mut lens.sellmeier_index,
                                        self.all_glasses.as_slice(),
                                        |(label, _lens)| std::borrow::Cow::Borrowed(label),
                                    ) {
                                        lens.sellmeier = self.all_glasses[lens.sellmeier_index].1;
                                        next_sellmeier = Some((
                                            self.all_glasses[lens.sellmeier_index].1,
                                            lens.sellmeier_index,
                                        ));
                                        update_lens = true;
                                    }
                                }
                            } else if let Some(sellmeier) = next_sellmeier {
                                lens.sellmeier = sellmeier.0;
                                lens.sellmeier_index = sellmeier.1;
                                next_sellmeier = None;
                            }
                            if ui_draw_next > 0 {
                                ui.same_line();
                                update_lens |=
                                    ui.checkbox(format!("spherical##{}", i), &mut lens.spherical);

                                update_lens |= Drag::new(format!("d##{}", i))
                                    .range(0., 500.)
                                    .speed(0.1)
                                    .build(ui, &mut lens.d);
                                ui.same_line();
                                update_lens |= Drag::new(format!("r##{}", i))
                                    .speed(0.01)
                                    .build(ui, &mut lens.r);
                                // update_lens |=
                                //     Slider::new(format!("d2##{}", i), -3., 6.).build(&ui, &mut lens.d2);
                                // ui.same_line();
                                // update_lens |=
                                //     Slider::new(format!("r2##{}", i), -6., 3.).build(&ui, &mut lens.r2);

                                // thickness: 0.1016260162601626, ior: 1.23
                                update_lens |= Slider::new(format!("wavelen##{}", i), 0.3, 0.8)
                                    .build(ui, &mut lens.coating_optimal);
                                ui.same_line();
                                update_lens |= ui
                                    .checkbox(format!("coating##{}", i), &mut lens.coating_enable);

                                if ui.button(format!("delete##{}", i)) {
                                    delete_glass = Some(i - 1);
                                }

                                // update_size |= ui.checkbox(format!("button##{}", i), &mut element.4);
                                // update_lens |= update_size;
                                if !lens.entry {
                                    ui.separator();
                                    ui.push_item_width(0.);
                                }

                                ui_draw_next -= 1;
                            }
                        }
                        ElementState::Aperture(aperture) => {
                            aperture_counter += 1;
                            if CollapsingHeader::new(format!("Aperture {:?}", aperture_counter))
                                .build(ui)
                            {
                                update_lens |= Drag::new(format!("d##{}", i))
                                    .range(0., 500.)
                                    .speed(0.1)
                                    .build(ui, &mut aperture.d);
                                update_lens |= Drag::new(format!("r1##{}", i))
                                    .range(0., 100.)
                                    .speed(0.01)
                                    .build(ui, &mut aperture.r);
                                update_lens |= Slider::new(format!("num_blades##{}", i), 3, 16)
                                    .build(ui, &mut aperture.num_blades);

                                if ui.button(format!("delete##{}", i)) {
                                    delete_aperture = Some(i);
                                }

                                ui.separator();
                            }
                        }
                    }
                }

                if let Some(delete_element) = delete_glass {
                    self.lens.remove(delete_element);
                    self.lens.remove(delete_element);
                    update_lens = true;
                }

                if let Some(delete_aperture) = delete_aperture {
                    self.lens.remove(delete_aperture);
                    update_lens = true;
                }

                if ui.button("add aperture") {
                    self.lens.push(ElementState::Aperture(Aperture {
                        d: 1.5,
                        r: 1.,
                        num_blades: 6,
                    }));
                    update_lens = true;
                }
                ui.same_line();
                if ui.button("add element") {
                    self.lens.push(ElementState::Lens(GlassElement {
                        d: 1.5,
                        r: 3.,
                        entry: true,
                        spherical: true,
                        sellmeier: Sellmeier::bk7(),
                        sellmeier_index: 0,
                        coating_optimal: 0.5,
                        coating_enable: false,
                    }));
                    self.lens.push(ElementState::Lens(GlassElement {
                        d: 1.5,
                        r: 3.,
                        entry: false,
                        spherical: true,
                        sellmeier: Sellmeier::bk7(),
                        sellmeier_index: 0,
                        coating_optimal: 0.5,
                        coating_enable: false,
                    }));
                    update_lens = true;
                }

                if ui.button("save as") {
                    if !self.current_filename.is_empty() {
                        if let Err(err) = self.save(self.current_filename.as_str()) {
                            println!("fuck, {:?}", err);
                        };
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

                update_sensor |= Drag::new("sensor distance")
                    .range(0., 469245.)
                    .speed(0.01)
                    .build(ui, &mut self.actual_lens.sensor_dist);
                update_lens |= Slider::new("zoom", 0., 10.).build(ui, &mut self.sim_params[12]);
                update_lens |=
                    Slider::new("num_wavelengths", 1, 20).build(ui, &mut self.num_wavelengths);
                update_lens |= update_sensor
            });

        let mut update_rays = self.first_frame;
        let mut update_dots = self.first_frame;
        let mut update_res = false;
        let mut render = false;
        let mut compute = false;

        let sample = 1. / (Instant::now() - self.last_frame_time).as_secs_f64();
        let alpha = 0.98;
        self.fps = alpha * self.fps + (1.0 - alpha) * sample;
        imgui::Window::new("Params")
            .size([400.0, 250.0], Condition::FirstUseEver)
            .position([600.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                let num_ghosts = (self
                    .actual_lens
                    .get_ghosts_indicies(self.draw as usize, 0)
                    .len()) as u32;
                if Slider::new("which ghost", 0, num_ghosts).build(ui, &mut self.which_ghost) {
                    update_lens = true;
                }
                ui.text(format!("Framerate: {:.0}", self.fps));
                update_rays |=
                    Slider::new("rays_exponent", 0., 6.5).build(ui, &mut self.ray_exponent);
                ui.text(format!("rays: {}", 10.0_f64.powf(self.ray_exponent) as u32));

                update_dots |=
                    Slider::new("dots_exponent", 0., if self.triangulate { 3. } else { 7.3 })
                        .build(ui, &mut self.dots_exponent);
                ui.text(format!(
                    "dots: {}",
                    10.0_f64.powf(self.dots_exponent) as u32
                ));

                update_lens |= Drag::new("opacity")
                    .range(0., 100.)
                    .speed(0.001)
                    .build(ui, &mut self.opacity);
                update_res |= Slider::new("scale_fact", 0.001, 1.).build(ui, &mut self.scale_fact);

                update_lens |= ui.radio_button("render both", &mut self.draw, 3)
                    || ui.radio_button("render normal", &mut self.draw, 2)
                    || ui.radio_button("render ghosts", &mut self.draw, 1);

                ui.text("render");
                ui.same_line();
                if ui.checkbox("triangulated", &mut self.triangulate) {
                    update_lens = true;
                    update_dots = true;
                }
                ui.same_line();
                if ui.checkbox("backgroud", &mut self.draw_background) {
                    update_lens = true;
                }

                // ui.radio_button("num_rays", &mut lens_ui.1, true);
                update_lens |= Drag::new("ray origin")
                    .speed(0.001)
                    .range(-10., 10.)
                    .build_array(ui, &mut self.pos_params[0..3]);

                update_lens |= Drag::new("ray direction")
                    .speed(0.001)
                    .range(-1., 1.)
                    .build_array(ui, &mut self.pos_params[4..7]);

                update_lens |= Drag::new("ray width")
                    .range(0., 10.)
                    .speed(0.01)
                    .build(ui, &mut self.pos_params[9]);

                render = ui.button("hi-res render");
                ui.same_line();
                compute = ui.button("compute");
                ui.same_line();
                if !self.triangulate {
                    Slider::new("num_hi_rays", 0., 12.).build(ui, &mut self.hi_dots_exponent);
                }
            });

        if update_lens || self.needs_update {
            self.update(device, queue);
            self.needs_update = false;
        }

        self.last_frame_time = Instant::now();

        self.first_frame = false;
        (update_lens, update_rays, update_dots, render, update_res, compute)
    }
}
