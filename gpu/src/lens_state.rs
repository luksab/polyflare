use std::fs::{self, DirBuilder};
use std::path::Path;
use std::time::Instant;

use cgmath::{InnerSpace, Vector3};
use directories::ProjectDirs;
use imgui::{Condition, Drag, Slider, Ui};
use polynomial_optics::{Element, Glass, Lens, Properties, Sellmeier};
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue};

impl Sensor {
    /// Leica M8
    /// from https://www.image-engineering.de/content/library/diploma_thesis/christian_mauer_spectral_response.pdf
    pub fn leica_m8() -> Self {
        let arr = vec![
            (380., 0.02, 0.01, 0.03),
            (390., 0.03, 0.02, 0.10),
            (400., 0.03, 0.04, 0.27),
            (410., 0.03, 0.04, 0.39),
            (420., 0.03, 0.05, 0.44),
            (430., 0.03, 0.06, 0.51),
            (440., 0.03, 0.09, 0.62),
            (450., 0.04, 0.12, 0.70),
            (460., 0.05, 0.18, 0.77),
            (470., 0.07, 0.30, 0.84),
            (480., 0.08, 0.40, 0.86),
            (490., 0.08, 0.44, 0.84),
            (500., 0.10, 0.60, 0.74),
            (510., 0.11, 0.75, 0.62),
            (520., 0.12, 0.93, 0.46),
            (530., 0.12, 1.03, 0.30),
            (540., 0.13, 1.02, 0.20),
            (550., 0.14, 1.00, 0.14),
            (560., 0.13, 0.92, 0.09),
            (570., 0.13, 0.82, 0.07),
            (580., 0.19, 0.67, 0.06),
            (590., 0.38, 0.51, 0.06),
            (600., 0.61, 0.30, 0.05),
            (610., 0.62, 0.20, 0.04),
            (620., 0.55, 0.12, 0.04),
            (630., 0.46, 0.08, 0.03),
            (640., 0.38, 0.06, 0.03),
            (650., 0.28, 0.04, 0.02),
            (660., 0.22, 0.03, 0.02),
            (670., 0.16, 0.03, 0.02),
            (680., 0.13, 0.03, 0.02),
            (690., 0.09, 0.03, 0.01),
            (700., 0.07, 0.03, 0.01),
            (710., 0.06, 0.02, 0.01),
            (720., 0.04, 0.02, 0.01),
            (750., 0.02, 0.01, 0.00),
            (800., 0.01, 0.01, 0.00),
            (850., 0.00, 0.00, 0.00),
            (905., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }

    pub fn nikon_d700() -> Self {
        let arr = vec![
            (380., 0.00, 0.01, 0.01),
            (390., 0.00, 0.00, 0.00),
            (400., 0.00, 0.00, 0.00),
            (410., 0.00, 0.00, 0.03),
            (420., 0.05, 0.02, 0.38),
            (430., 0.07, 0.04, 0.70),
            (440., 0.05, 0.06, 0.87),
            (450., 0.04, 0.09, 1.04),
            (460., 0.03, 0.13, 1.02),
            (470., 0.04, 0.26, 1.01),
            (480., 0.04, 0.39, 0.90),
            (490., 0.04, 0.43, 0.78),
            (500., 0.04, 0.60, 0.56),
            (510., 0.04, 0.81, 0.32),
            (520., 0.07, 0.98, 0.17),
            (530., 0.08, 1.07, 0.07),
            (540., 0.05, 1.06, 0.04),
            (550., 0.03, 1.00, 0.02),
            (560., 0.03, 0.87, 0.01),
            (570., 0.09, 0.68, 0.01),
            (580., 0.48, 0.48, 0.00),
            (590., 0.76, 0.33, 0.00),
            (600., 0.74, 0.15, 0.00),
            (610., 0.64, 0.07, 0.00),
            (620., 0.54, 0.03, 0.00),
            (630., 0.44, 0.02, 0.00),
            (640., 0.35, 0.01, 0.00),
            (650., 0.25, 0.01, 0.00),
            (660., 0.20, 0.01, 0.00),
            (670., 0.11, 0.00, 0.00),
            (680., 0.04, 0.00, 0.00),
            (690., 0.01, 0.00, 0.00),
            (700., 0.00, 0.00, 0.00),
            (710., 0.00, 0.00, 0.00),
            (720., 0.00, 0.00, 0.00),
            (750., 0.00, 0.00, 0.00),
            (800., 0.00, 0.00, 0.00),
            (850., 0.00, 0.00, 0.00),
            (905., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
}

    pub fn canon_eos_40d() -> Self {
        let arr = vec![
            (380., 0.01, 0.01, 0.01),
            (390., 0.00, 0.00, 0.01),
            (400., 0.00, 0.00, 0.01),
            (410., 0.00, 0.01, 0.09),
            (420., 0.00, 0.03, 0.42),
            (430., 0.00, 0.05, 0.65),
            (440., 0.00, 0.08, 0.87),
            (450., 0.00, 0.09, 0.96),
            (460., 0.00, 0.13, 1.01),
            (470., 0.01, 0.29, 0.97),
            (480., 0.01, 0.54, 0.90),
            (490., 0.01, 0.70, 0.81),
            (500., 0.02, 0.94, 0.61),
            (510., 0.04, 1.01, 0.43),
            (520., 0.08, 1.08, 0.25),
            (530., 0.14, 1.06, 0.14),
            (540., 0.15, 1.01, 0.10),
            (550., 0.14, 1.00, 0.07),
            (560., 0.17, 0.87, 0.05),
            (570., 0.30, 0.75, 0.04),
            (580., 0.45, 0.58, 0.03),
            (590., 0.53, 0.42, 0.02),
            (600., 0.56, 0.24, 0.01),
            (610., 0.53, 0.15, 0.01),
            (620., 0.46, 0.08, 0.01),
            (630., 0.38, 0.05, 0.01),
            (640., 0.33, 0.03, 0.01),
            (650., 0.23, 0.02, 0.01),
            (660., 0.19, 0.02, 0.01),
            (670., 0.15, 0.02, 0.01),
            (680., 0.10, 0.01, 0.00),
            (690., 0.03, 0.01, 0.00),
            (700., 0.00, 0.00, 0.00),
            (710., 0.00, 0.00, 0.00),
            (720., 0.00, 0.00, 0.00),
            (750., 0.00, 0.00, 0.00),
            (800., 0.00, 0.00, 0.00),
            (850., 0.00, 0.00, 0.00),
            (905., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }

    pub fn fuji_s5_pro() -> Self {
        let arr = vec![
            (380., 0.01, 0.01, 0.01),
            (390., 0.00, 0.01, 0.03),
            (400., 0.01, 0.01, 0.25),
            (410., 0.01, 0.01, 0.44),
            (420., 0.01, 0.01, 0.62),
            (430., 0.00, 0.00, 0.74),
            (440., 0.00, 0.01, 0.86),
            (450., 0.00, 0.01, 0.99),
            (460., 0.00, 0.01, 1.02),
            (470., 0.00, 0.05, 1.01),
            (480., 0.01, 0.37, 0.95),
            (490., 0.01, 0.68, 0.84),
            (500., 0.01, 0.98, 0.63),
            (510., 0.01, 1.07, 0.47),
            (520., 0.02, 1.09, 0.29),
            (530., 0.03, 1.10, 0.14),
            (540., 0.02, 1.08, 0.09),
            (550., 0.01, 1.00, 0.05),
            (560., 0.01, 0.88, 0.02),
            (570., 0.05, 0.75, 0.01),
            (580., 0.31, 0.58, 0.01),
            (590., 0.83, 0.39, 0.01),
            (600., 0.90, 0.23, 0.01),
            (610., 0.86, 0.09, 0.01),
            (620., 0.79, 0.05, 0.01),
            (630., 0.72, 0.02, 0.01),
            (640., 0.70, 0.02, 0.01),
            (650., 0.60, 0.01, 0.01),
            (660., 0.54, 0.01, 0.02),
            (670., 0.41, 0.01, 0.02),
            (680., 0.20, 0.01, 0.01),
            (690., 0.05, 0.01, 0.00),
            (700., 0.02, 0.00, 0.00),
            (710., 0.01, 0.00, 0.00),
            (720., 0.00, 0.00, 0.00),
            (750., 0.00, 0.00, 0.00),
            (800., 0.00, 0.00, 0.00),
            (850., 0.00, 0.00, 0.00),
            (900., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }

    pub fn panasonic_dmc_lx3() -> Self {
        let arr = vec![
            (380., 0.00, 0.00, 0.00),
            (390., 0.00, 0.00, 0.00),
            (400., 0.00, 0.00, 0.00),
            (410., 0.00, 0.00, 0.02),
            (420., 0.01, 0.05, 0.18),
            (430., 0.03, 0.14, 0.54),
            (440., 0.04, 0.28, 0.95),
            (450., 0.03, 0.34, 1.03),
            (460., 0.03, 0.41, 1.01),
            (470., 0.03, 0.55, 0.98),
            (480., 0.03, 0.62, 0.87),
            (490., 0.03, 0.65, 0.79),
            (500., 0.04, 0.83, 0.59),
            (510., 0.05, 1.01, 0.46),
            (520., 0.08, 1.10, 0.31),
            (530., 0.09, 1.13, 0.21),
            (540., 0.06, 1.05, 0.15),
            (550., 0.04, 1.00, 0.10),
            (560., 0.04, 0.88, 0.06),
            (570., 0.04, 0.73, 0.04),
            (580., 0.23, 0.58, 0.03),
            (590., 0.65, 0.44, 0.03),
            (600., 0.74, 0.28, 0.02),
            (610., 0.70, 0.20, 0.02),
            (620., 0.63, 0.14, 0.02),
            (630., 0.56, 0.10, 0.02),
            (640., 0.51, 0.08, 0.02),
            (650., 0.42, 0.06, 0.03),
            (660., 0.30, 0.05, 0.02),
            (670., 0.15, 0.03, 0.02),
            (680., 0.06, 0.02, 0.01),
            (690., 0.01, 0.00, 0.00),
            (700., 0.00, 0.00, 0.00),
            (710., 0.00, 0.00, 0.00),
            (720., 0.00, 0.00, 0.00),
            (750., 0.00, 0.00, 0.00),
            (800., 0.00, 0.00, 0.00),
            (850., 0.00, 0.00, 0.00),
            (905., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }

    pub fn arriflex_d_21() -> Self {
        let arr = vec![
            (380., 0.00, 0.00, 0.00),
            (390., 0.02, 0.02, 0.03),
            (400., 0.05, 0.02, 0.07),
            (410., 0.15, 0.04, 0.32),
            (420., 0.14, 0.05, 0.48),
            (430., 0.12, 0.06, 0.68),
            (440., 0.06, 0.08, 0.85),
            (450., 0.05, 0.13, 0.99),
            (460., 0.06, 0.18, 1.11),
            (470., 0.07, 0.27, 1.01),
            (480., 0.08, 0.34, 0.93),
            (490., 0.09, 0.37, 0.79),
            (500., 0.11, 0.47, 0.54),
            (510., 0.21, 0.79, 0.44),
            (520., 0.30, 1.02, 0.34),
            (530., 0.40, 1.10, 0.28),
            (540., 0.44, 1.15, 0.26),
            (550., 0.38, 1.00, 0.22),
            (560., 0.44, 0.85, 0.19),
            (570., 0.63, 0.55, 0.13),
            (580., 0.92, 0.36, 0.10),
            (590., 1.07, 0.23, 0.08),
            (600., 1.02, 0.13, 0.06),
            (610., 0.80, 0.09, 0.05),
            (620., 0.74, 0.08, 0.04),
            (630., 0.53, 0.05, 0.03),
            (640., 0.38, 0.05, 0.03),
            (650., 0.23, 0.03, 0.02),
            (660., 0.16, 0.03, 0.02),
            (670., 0.11, 0.03, 0.02),
            (680., 0.05, 0.01, 0.01),
            (690., 0.03, 0.01, 0.01),
            (700., 0.01, 0.00, 0.01),
            (710., 0.01, 0.00, 0.01),
            (720., 0.01, 0.00, 0.01),
            (750., 0.00, 0.00, 0.00),
            (800., 0.00, 0.00, 0.00),
            (850., 0.01, 0.01, 0.01),
            (905., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }

    pub fn canon_eos_450_d_no_ir_filter() -> Self {
        let arr = vec![
            (380., 0.04, 0.02, 0.04),
            (390., 0.04, 0.02, 0.11),
            (400., 0.03, 0.03, 0.24),
            (410., 0.02, 0.03, 0.34),
            (420., 0.01, 0.04, 0.46),
            (430., 0.00, 0.04, 0.50),
            (440., 0.00, 0.06, 0.64),
            (450., 0.00, 0.08, 0.71),
            (460., 0.01, 0.13, 0.77),
            (470., 0.01, 0.26, 0.75),
            (480., 0.02, 0.51, 0.73),
            (490., 0.02, 0.64, 0.67),
            (500., 0.04, 0.81, 0.50),
            (510., 0.06, 0.92, 0.39),
            (520., 0.07, 1.02, 0.26),
            (530., 0.09, 1.01, 0.16),
            (540., 0.12, 1.03, 0.13),
            (550., 0.17, 1.00, 0.10),
            (560., 0.23, 0.92, 0.07),
            (570., 0.37, 0.84, 0.06),
            (580., 0.59, 0.78, 0.06),
            (590., 0.78, 0.58, 0.05),
            (600., 0.77, 0.39, 0.03),
            (610., 0.80, 0.21, 0.02),
            (620., 0.87, 0.15, 0.02),
            (630., 0.82, 0.10, 0.02),
            (640., 0.77, 0.08, 0.02),
            (650., 0.83, 0.08, 0.03),
            (660., 0.77, 0.07, 0.04),
            (670., 0.69, 0.09, 0.04),
            (680., 0.65, 0.11, 0.04),
            (690., 0.60, 0.15, 0.05),
            (700., 0.62, 0.19, 0.05),
            (710., 0.63, 0.21, 0.05),
            (720., 0.58, 0.19, 0.04),
            (750., 0.53, 0.22, 0.03),
            (800., 0.38, 0.30, 0.20),
            (850., 0.23, 0.23, 0.23),
            (905., 0.12, 0.13, 0.12),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }

    pub fn hasselblad_h3_d() -> Self {
        let arr = vec![
            (380., 0.06, 0.02, 0.03),
            (390., 0.10, 0.03, 0.13),
            (400., 0.11, 0.04, 0.25),
            (410., 0.11, 0.05, 0.36),
            (420., 0.12, 0.06, 0.44),
            (430., 0.13, 0.08, 0.55),
            (440., 0.14, 0.11, 0.67),
            (450., 0.16, 0.16, 0.75),
            (460., 0.18, 0.19, 0.79),
            (470., 0.24, 0.28, 0.76),
            (480., 0.28, 0.35, 0.81),
            (490., 0.29, 0.39, 0.75),
            (500., 0.33, 0.49, 0.72),
            (510., 0.40, 0.66, 0.65),
            (520., 0.46, 0.80, 0.55),
            (530., 0.53, 1.03, 0.44),
            (540., 0.62, 1.07, 0.34),
            (550., 0.60, 1.00, 0.24),
            (560., 0.55, 0.90, 0.16),
            (570., 0.55, 0.74, 0.13),
            (580., 0.65, 0.66, 0.12),
            (590., 1.11, 0.44, 0.09),
            (600., 1.55, 0.27, 0.08),
            (610., 1.34, 0.16, 0.06),
            (620., 1.05, 0.10, 0.04),
            (630., 0.93, 0.07, 0.04),
            (640., 0.63, 0.05, 0.03),
            (650., 0.29, 0.02, 0.02),
            (660., 0.20, 0.02, 0.01),
            (670., 0.16, 0.01, 0.01),
            (680., 0.09, 0.01, 0.01),
            (690., 0.04, 0.01, 0.00),
            (700., 0.02, 0.00, 0.00),
            (710., 0.01, 0.00, 0.00),
            (720., 0.00, 0.00, 0.00),
            (750., 0.00, 0.00, 0.00),
            (800., 0.00, 0.00, 0.00),
            (850., 0.00, 0.00, 0.00),
            (906., 0.00, 0.00, 0.00),
        ];
        Self {
            measuremens: arr
                .iter()
                .map(|(wl, r, g, b)| SensorDatapoint {
                    rgb: Vector3 {
                        x: *r,
                        y: *g,
                        z: *b,
                    },
                    wavelength: *wl as f32,
                })
                .collect(),
        }
    }
}


/// How much a single wavelength influences r, g and b of the Sensor
pub struct SensorDatapoint {
    pub rgb: Vector3<f32>,
    pub wavelength: f32,
}

/// A representation of a sensor - given by measurements of a particular wavelength
/// Measurements taken from https://www.image-engineering.de/content/library/diploma_thesis/christian_mauer_spectral_response.pdf
pub struct Sensor {
    pub measuremens: Vec<SensorDatapoint>,
}

impl Sensor {
    /// get the contained data in r,g,b,wavelength format
    pub fn get_data(&self) -> Vec<f32> {
        let mut result = vec![];
        for measurement in &self.measuremens {
            result.push(measurement.rgb.x);
            result.push(measurement.rgb.y);
            result.push(measurement.rgb.z);
            result.push(measurement.wavelength);
        }
        result
    }
}

pub struct GlassElement {
    d1: f32,
    r1: f32,
    d2: f32,
    r2: f32,
    sellmeier: Sellmeier,
}

pub struct Aperture {
    d: f32,
    r: f32,
    num_blades: u32,
}

pub enum ElementState {
    Lens(GlassElement),
    Aperture(Aperture),
}

pub struct LensState {
    pub ray_exponent: f64,
    pub dots_exponent: f64,
    pub hi_dots_exponent: f64,
    pub draw: u32,
    pub opacity: f32,
    pub which_ghost: u32,

    lens: Vec<ElementState>,
    /// The actual Lens being rendered
    pub actual_lens: Lens,
    selected_lens: usize,
    current_filename: String,

    sensor: Sensor,
    pub sensor_buffer: Buffer,
    /// positions of the rays and the sensor
    pub pos_params_buffer: wgpu::Buffer,
    pub pos_bind_group: wgpu::BindGroup,
    pub pos_bind_group_layout: wgpu::BindGroupLayout,
    /// Data for the positions of the rays and the sensor
    pub pos_params: [f32; 12],

    lens_buffer: Buffer,
    lens_rt_buffer: Buffer,
    pub lens_bind_group: wgpu::BindGroup,
    pub lens_bind_group_layout: wgpu::BindGroupLayout,

    last_frame_time: Instant,
    fps: f64,

    first_frame: bool,
}

impl LensState {
    pub fn default(device: &Device) -> Self {
        let lens = vec![
            ElementState::Lens(GlassElement {
                d1: 0.,
                r1: 3.,
                d2: 1.5,
                r2: 3.,
                sellmeier: Sellmeier::BK7(),
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
                sellmeier: Sellmeier::BK7(),
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

        let sensor = Sensor::leica_m8();
        let sensor_data = sensor.get_data();

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
            sensor,
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
                            spherical: true,
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
    pub fn get_lens(&self) -> Vec<polynomial_optics::Element> {
        Self::get_lens_arr(&self.lens)
    }

    fn get_lens_state(lens: &Lens) -> Vec<ElementState> {
        let mut elements = vec![];
        let mut last_pos = -5.;
        let mut expect_entry = true;
        let mut enty = (0., 0.);

        for element in &lens.elements {
            match element.properties {
                Properties::Glass(glass) => {
                    if expect_entry && glass.entry {
                        enty = (element.position as f32 - last_pos, element.radius as f32);
                        expect_entry = false;
                    } else if !expect_entry && !glass.entry {
                        elements.push(ElementState::Lens(GlassElement {
                            d1: enty.0,
                            r1: enty.1,
                            d2: element.position as f32 - last_pos,
                            r2: element.radius as f32,
                            sellmeier: glass.sellmeier,
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
                    self.lens = Self::get_lens_state(&mut self.actual_lens);
                    self.current_filename = lenses[self.selected_lens].0.clone();
                    update_lens = true;
                }

                for (i, element) in self.lens.iter_mut().enumerate() {
                    match element {
                        ElementState::Lens(lens) => {
                            ui.text(format!("Lens: {:?}", i + 1));
                            ui.push_item_width(ui.window_size()[0] / 2. - 45.);
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
                if ui.button("save lens") {
                    Self::save_lens(lenses[self.selected_lens].0.as_str(), &self.actual_lens);
                }
                ui.same_line();
                if ui.button("save as") {
                    Self::save_lens(self.current_filename.as_str(), &self.actual_lens);
                }
                ui.same_line();
                ui.input_text("filename", &mut self.current_filename)
                    .build();

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
