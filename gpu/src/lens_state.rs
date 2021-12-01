use std::time::Instant;

use imgui::{Condition, Drag, Slider, Ui};
use polynomial_optics::{Element, Glass, Properties};

pub struct Lens {
    d1: f32,
    r1: f32,
    d2: f32,
    r2: f32,
    ior: f64,
}

pub struct Aperture {
    d: f32,
    r: f32,
    num_blades: u32,
}

pub enum ElementState {
    Lens(Lens),
    Aperture(Aperture),
}

pub struct LensState {
    pub ray_exponent: f64,
    pub dots_exponent: f64,
    pub draw: u32,
    pub pos: [f32; 3],
    pub dir: [f32; 3],
    pub opacity: f32,
    pub sensor_dist: f32,
    pub which_ghost: u32,

    lens: Vec<ElementState>,

    last_frame_time: Instant,
}

impl Default for LensState {
    fn default() -> Self {
        let lens = vec![
            ElementState::Lens(Lens {
                d1: 0.,
                r1: 3.,
                d2: 1.5,
                r2: 3.,
                ior: 1.5,
            }),
            ElementState::Aperture(Aperture {
                d: 1.5,
                r: 1.,
                num_blades: 6,
            }),
            ElementState::Lens(Lens {
                d1: 0.,
                r1: 3.,
                d2: 1.5,
                r2: 3.,
                ior: 1.5,
            }),
        ];
        Self {
            ray_exponent: 2.7,
            dots_exponent: 7.,
            draw: 1,
            pos: [0.0, 0.0, -7.0],
            dir: [0.0, 0.0, 1.0],
            opacity: 0.75,
            sensor_dist: 3.,
            which_ghost: 0,
            lens,
            last_frame_time: Instant::now(),
        }
    }
}

impl LensState {
    pub fn get_lens(&self) -> Vec<polynomial_optics::Element> {
        let mut elements: Vec<Element> = vec![];
        let mut dst: f32 = -5.;
        for element in &self.lens {
            match element {
                ElementState::Lens(lens) => {
                    dst += lens.d1;
                    elements.push(Element {
                        radius: lens.r1 as f64,
                        properties: Properties::Glass(Glass {
                            ior: lens.ior,
                            coating: (),
                            entry: true,
                            spherical: true,
                        }),
                        position: dst as f64,
                    });
                    dst += lens.d2;
                    elements.push(Element {
                        radius: lens.r2 as f64,
                        properties: Properties::Glass(Glass {
                            ior: lens.ior,
                            coating: (),
                            entry: false,
                            spherical: true,
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

    /// create an imgui window from Self and return
    ///
    /// (update_lens, update_sensor, update_lens_size, update_ray_num)
    pub fn build_ui(&mut self, ui: &Ui) -> (bool, bool, bool, bool) {
        let mut update_lens = false;
        let mut update_sensor = false;
        imgui::Window::new("Lens")
            .size([400.0, 250.0], Condition::FirstUseEver)
            .position([100.0, 100.0], Condition::FirstUseEver)
            .build(ui, || {
                let num_ghosts = (self.lens.len() * self.lens.len()) as u32;
                update_lens |=
                    Slider::new("which ghost", 0, num_ghosts).build(&ui, &mut self.which_ghost);
                for (i, element) in self.lens.iter_mut().enumerate() {
                    match element {
                        ElementState::Lens(lens) => {
                            ui.text(format!("Lens: {:?}", i + 1));
                            update_lens |=
                                Slider::new(format!("d##{}", i), 0., 5.).build(&ui, &mut lens.d1);
                            update_lens |=
                                Slider::new(format!("r1##{}", i), -3., 3.).build(&ui, &mut lens.r1);
                            update_lens |=
                                Slider::new(format!("r2##{}", i), -3., 3.).build(&ui, &mut lens.r2);
                            update_lens |= Slider::new(format!("d_next##{}", i), -3., 6.)
                                .build(&ui, &mut lens.d2);

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
                update_sensor |=
                    Slider::new("sensor distance", 0., 20.).build(&ui, &mut self.sensor_dist);
            });

        let mut update_rays = false;
        imgui::Window::new("Params")
            .size([400.0, 250.0], Condition::FirstUseEver)
            .position([600.0, 100.0], Condition::FirstUseEver)
            .build(&ui, || {
                ui.text(format!(
                    "Framerate: {:?}",
                    1. / (Instant::now() - self.last_frame_time).as_secs_f64()
                ));
                update_rays |=
                    Slider::new("rays_exponent", 0., 6.5).build(&ui, &mut self.ray_exponent);
                ui.text(format!("rays: {}", 10.0_f64.powf(self.ray_exponent) as u32));

                update_rays |=
                    Slider::new("dots_exponent", 0., 10.).build(&ui, &mut self.dots_exponent);
                ui.text(format!(
                    "dots: {}",
                    10.0_f64.powf(self.dots_exponent) as u32
                ));

                update_lens |= Slider::new("opacity", 0., 1.).build(&ui, &mut self.opacity);

                update_rays |= ui.radio_button("render nothing", &mut self.draw, 0)
                    || ui.radio_button("render both", &mut self.draw, 3)
                    || ui.radio_button("render normal", &mut self.draw, 2)
                    || ui.radio_button("render ghosts", &mut self.draw, 1);

                // ui.radio_button("num_rays", &mut lens_ui.1, true);
                update_lens |= Drag::new("ray origin")
                    .speed(0.01)
                    .range(-10., 10.)
                    .build_array(&ui, &mut self.pos);

                update_lens |= Drag::new("ray direction")
                    .speed(0.01)
                    .range(-1., 1.)
                    .build_array(&ui, &mut self.dir);
            });
        self.last_frame_time = Instant::now();
        (update_lens, update_sensor, false, update_lens)
    }
}
