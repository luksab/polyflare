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
    /// from https://www.image-engineering.de/content/library/diploma_thesis/christian_mauer_spectral_response.pdf
    #[rustfmt::skip]
    fn get_all_sensors() -> Vec<(String, Sensor)> {
        vec![
            ("Leica M8".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.02, y: 0.01, z: 0.03} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.03, y: 0.02, z: 0.10} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.03, y: 0.04, z: 0.27} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.03, y: 0.04, z: 0.39} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.03, y: 0.05, z: 0.44} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.03, y: 0.06, z: 0.51} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.03, y: 0.09, z: 0.62} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.04, y: 0.12, z: 0.70} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.05, y: 0.18, z: 0.77} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.07, y: 0.30, z: 0.84} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.08, y: 0.40, z: 0.86} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.08, y: 0.44, z: 0.84} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.10, y: 0.60, z: 0.74} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.11, y: 0.75, z: 0.62} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.12, y: 0.93, z: 0.46} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.12, y: 1.03, z: 0.30} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.13, y: 1.02, z: 0.20} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.14, y: 1.00, z: 0.14} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.13, y: 0.92, z: 0.09} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.13, y: 0.82, z: 0.07} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.19, y: 0.67, z: 0.06} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.38, y: 0.51, z: 0.06} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.61, y: 0.30, z: 0.05} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.62, y: 0.20, z: 0.04} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.55, y: 0.12, z: 0.04} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.46, y: 0.08, z: 0.03} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.38, y: 0.06, z: 0.03} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.28, y: 0.04, z: 0.02} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.22, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.16, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.13, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.09, y: 0.03, z: 0.01} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.07, y: 0.03, z: 0.01} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.06, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.04, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.02, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 905., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Nikon D700".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.00, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.03} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.05, y: 0.02, z: 0.38} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.07, y: 0.04, z: 0.70} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.05, y: 0.06, z: 0.87} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.04, y: 0.09, z: 1.04} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.03, y: 0.13, z: 1.02} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.04, y: 0.26, z: 1.01} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.04, y: 0.39, z: 0.90} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.04, y: 0.43, z: 0.78} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.04, y: 0.60, z: 0.56} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.04, y: 0.81, z: 0.32} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.07, y: 0.98, z: 0.17} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.08, y: 1.07, z: 0.07} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.05, y: 1.06, z: 0.04} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.03, y: 1.00, z: 0.02} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.03, y: 0.87, z: 0.01} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.09, y: 0.68, z: 0.01} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.48, y: 0.48, z: 0.00} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.76, y: 0.33, z: 0.00} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.74, y: 0.15, z: 0.00} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.64, y: 0.07, z: 0.00} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.54, y: 0.03, z: 0.00} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.44, y: 0.02, z: 0.00} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.35, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.25, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.20, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.11, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.04, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 905., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Canon EOS 40D".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.01} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.01} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.00, y: 0.01, z: 0.09} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.00, y: 0.03, z: 0.42} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.00, y: 0.05, z: 0.65} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.00, y: 0.08, z: 0.87} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.00, y: 0.09, z: 0.96} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.00, y: 0.13, z: 1.01} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.01, y: 0.29, z: 0.97} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.01, y: 0.54, z: 0.90} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.01, y: 0.70, z: 0.81} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.02, y: 0.94, z: 0.61} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.04, y: 1.01, z: 0.43} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.08, y: 1.08, z: 0.25} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.14, y: 1.06, z: 0.14} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.15, y: 1.01, z: 0.10} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.14, y: 1.00, z: 0.07} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.17, y: 0.87, z: 0.05} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.30, y: 0.75, z: 0.04} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.45, y: 0.58, z: 0.03} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.53, y: 0.42, z: 0.02} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.56, y: 0.24, z: 0.01} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.53, y: 0.15, z: 0.01} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.46, y: 0.08, z: 0.01} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.38, y: 0.05, z: 0.01} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.33, y: 0.03, z: 0.01} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.23, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.19, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.15, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.10, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.03, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 905., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Fuji S5 Pro".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.00, y: 0.01, z: 0.03} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.25} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.44} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.62} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.74} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.00, y: 0.01, z: 0.86} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.00, y: 0.01, z: 0.99} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.00, y: 0.01, z: 1.02} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.00, y: 0.05, z: 1.01} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.01, y: 0.37, z: 0.95} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.01, y: 0.68, z: 0.84} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.01, y: 0.98, z: 0.63} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.01, y: 1.07, z: 0.47} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.02, y: 1.09, z: 0.29} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.03, y: 1.10, z: 0.14} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.02, y: 1.08, z: 0.09} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.01, y: 1.00, z: 0.05} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.01, y: 0.88, z: 0.02} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.05, y: 0.75, z: 0.01} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.31, y: 0.58, z: 0.01} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.83, y: 0.39, z: 0.01} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.90, y: 0.23, z: 0.01} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.86, y: 0.09, z: 0.01} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.79, y: 0.05, z: 0.01} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.72, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.70, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.60, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.54, y: 0.01, z: 0.02} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.41, y: 0.01, z: 0.02} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.20, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.05, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.02, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 900., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Panasonic DMC-LX3".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.02} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.01, y: 0.05, z: 0.18} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.03, y: 0.14, z: 0.54} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.04, y: 0.28, z: 0.95} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.03, y: 0.34, z: 1.03} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.03, y: 0.41, z: 1.01} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.03, y: 0.55, z: 0.98} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.03, y: 0.62, z: 0.87} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.03, y: 0.65, z: 0.79} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.04, y: 0.83, z: 0.59} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.05, y: 1.01, z: 0.46} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.08, y: 1.10, z: 0.31} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.09, y: 1.13, z: 0.21} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.06, y: 1.05, z: 0.15} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.04, y: 1.00, z: 0.10} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.04, y: 0.88, z: 0.06} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.04, y: 0.73, z: 0.04} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.23, y: 0.58, z: 0.03} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.65, y: 0.44, z: 0.03} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.74, y: 0.28, z: 0.02} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.70, y: 0.20, z: 0.02} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.63, y: 0.14, z: 0.02} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.56, y: 0.10, z: 0.02} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.51, y: 0.08, z: 0.02} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.42, y: 0.06, z: 0.03} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.30, y: 0.05, z: 0.02} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.15, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.06, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 905., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Arriflex D-21".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.02, y: 0.02, z: 0.03} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.05, y: 0.02, z: 0.07} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.15, y: 0.04, z: 0.32} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.14, y: 0.05, z: 0.48} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.12, y: 0.06, z: 0.68} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.06, y: 0.08, z: 0.85} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.05, y: 0.13, z: 0.99} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.06, y: 0.18, z: 1.11} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.07, y: 0.27, z: 1.01} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.08, y: 0.34, z: 0.93} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.09, y: 0.37, z: 0.79} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.11, y: 0.47, z: 0.54} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.21, y: 0.79, z: 0.44} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.30, y: 1.02, z: 0.34} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.40, y: 1.10, z: 0.28} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.44, y: 1.15, z: 0.26} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.38, y: 1.00, z: 0.22} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.44, y: 0.85, z: 0.19} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.63, y: 0.55, z: 0.13} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.92, y: 0.36, z: 0.10} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 1.07, y: 0.23, z: 0.08} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 1.02, y: 0.13, z: 0.06} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.80, y: 0.09, z: 0.05} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.74, y: 0.08, z: 0.04} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.53, y: 0.05, z: 0.03} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.38, y: 0.05, z: 0.03} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.23, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.16, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.11, y: 0.03, z: 0.02} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.05, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.03, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.01} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.01} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.01} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 905., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Canon EOS 450D no IR filter".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.04, y: 0.02, z: 0.04} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.04, y: 0.02, z: 0.11} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.03, y: 0.03, z: 0.24} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.02, y: 0.03, z: 0.34} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.01, y: 0.04, z: 0.46} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.00, y: 0.04, z: 0.50} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.00, y: 0.06, z: 0.64} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.00, y: 0.08, z: 0.71} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.01, y: 0.13, z: 0.77} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.01, y: 0.26, z: 0.75} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.02, y: 0.51, z: 0.73} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.02, y: 0.64, z: 0.67} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.04, y: 0.81, z: 0.50} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.06, y: 0.92, z: 0.39} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.07, y: 1.02, z: 0.26} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.09, y: 1.01, z: 0.16} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.12, y: 1.03, z: 0.13} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.17, y: 1.00, z: 0.10} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.23, y: 0.92, z: 0.07} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.37, y: 0.84, z: 0.06} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.59, y: 0.78, z: 0.06} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.78, y: 0.58, z: 0.05} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.77, y: 0.39, z: 0.03} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.80, y: 0.21, z: 0.02} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.87, y: 0.15, z: 0.02} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.82, y: 0.10, z: 0.02} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.77, y: 0.08, z: 0.02} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.83, y: 0.08, z: 0.03} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.77, y: 0.07, z: 0.04} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.69, y: 0.09, z: 0.04} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.65, y: 0.11, z: 0.04} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.60, y: 0.15, z: 0.05} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.62, y: 0.19, z: 0.05} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.63, y: 0.21, z: 0.05} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.58, y: 0.19, z: 0.04} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.53, y: 0.22, z: 0.03} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.38, y: 0.30, z: 0.20} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.23, y: 0.23, z: 0.23} },
                    SensorDatapoint { wavelength: 905., rgb: Vector3 { x: 0.12, y: 0.13, z: 0.12} },
                ],
            }
        ),(
            "Hasselblad H3D".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.06, y: 0.02, z: 0.03} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.10, y: 0.03, z: 0.13} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.11, y: 0.04, z: 0.25} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.11, y: 0.05, z: 0.36} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.12, y: 0.06, z: 0.44} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.13, y: 0.08, z: 0.55} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.14, y: 0.11, z: 0.67} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.16, y: 0.16, z: 0.75} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.18, y: 0.19, z: 0.79} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.24, y: 0.28, z: 0.76} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.28, y: 0.35, z: 0.81} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.29, y: 0.39, z: 0.75} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.33, y: 0.49, z: 0.72} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.40, y: 0.66, z: 0.65} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.46, y: 0.80, z: 0.55} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.53, y: 1.03, z: 0.44} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.62, y: 1.07, z: 0.34} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.60, y: 1.00, z: 0.24} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.55, y: 0.90, z: 0.16} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.55, y: 0.74, z: 0.13} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.65, y: 0.66, z: 0.12} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 1.11, y: 0.44, z: 0.09} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 1.55, y: 0.27, z: 0.08} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 1.34, y: 0.16, z: 0.06} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 1.05, y: 0.10, z: 0.04} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.93, y: 0.07, z: 0.04} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.63, y: 0.05, z: 0.03} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.29, y: 0.02, z: 0.02} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.20, y: 0.02, z: 0.01} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.16, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.09, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.04, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.02, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 710., rgb: Vector3 { x: 0.01, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 720., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 906., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        ),(
            "Nikon D200".to_string(),
            Sensor {
                measuremens: vec![
                    SensorDatapoint { wavelength: 380., rgb: Vector3 { x: 0.02, y: 0.02, z: 0.03} },
                    SensorDatapoint { wavelength: 390., rgb: Vector3 { x: 0.02, y: 0.01, z: 0.02} },
                    SensorDatapoint { wavelength: 400., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.01} },
                    SensorDatapoint { wavelength: 410., rgb: Vector3 { x: 0.01, y: 0.01, z: 0.04} },
                    SensorDatapoint { wavelength: 420., rgb: Vector3 { x: 0.05, y: 0.05, z: 0.45} },
                    SensorDatapoint { wavelength: 430., rgb: Vector3 { x: 0.06, y: 0.08, z: 0.72} },
                    SensorDatapoint { wavelength: 440., rgb: Vector3 { x: 0.04, y: 0.10, z: 0.86} },
                    SensorDatapoint { wavelength: 450., rgb: Vector3 { x: 0.04, y: 0.14, z: 1.04} },
                    SensorDatapoint { wavelength: 460., rgb: Vector3 { x: 0.03, y: 0.18, z: 1.03} },
                    SensorDatapoint { wavelength: 470., rgb: Vector3 { x: 0.03, y: 0.31, z: 1.01} },
                    SensorDatapoint { wavelength: 480., rgb: Vector3 { x: 0.04, y: 0.42, z: 0.95} },
                    SensorDatapoint { wavelength: 490., rgb: Vector3 { x: 0.04, y: 0.47, z: 0.68} },
                    SensorDatapoint { wavelength: 500., rgb: Vector3 { x: 0.03, y: 0.59, z: 0.49} },
                    SensorDatapoint { wavelength: 510., rgb: Vector3 { x: 0.05, y: 0.88, z: 0.29} },
                    SensorDatapoint { wavelength: 520., rgb: Vector3 { x: 0.07, y: 1.05, z: 0.14} },
                    SensorDatapoint { wavelength: 530., rgb: Vector3 { x: 0.08, y: 1.05, z: 0.05} },
                    SensorDatapoint { wavelength: 540., rgb: Vector3 { x: 0.04, y: 1.10, z: 0.02} },
                    SensorDatapoint { wavelength: 550., rgb: Vector3 { x: 0.02, y: 1.00, z: 0.01} },
                    SensorDatapoint { wavelength: 560., rgb: Vector3 { x: 0.03, y: 0.87, z: 0.00} },
                    SensorDatapoint { wavelength: 570., rgb: Vector3 { x: 0.13, y: 0.72, z: 0.00} },
                    SensorDatapoint { wavelength: 580., rgb: Vector3 { x: 0.44, y: 0.53, z: 0.00} },
                    SensorDatapoint { wavelength: 590., rgb: Vector3 { x: 0.90, y: 0.32, z: 0.00} },
                    SensorDatapoint { wavelength: 600., rgb: Vector3 { x: 0.90, y: 0.16, z: 0.00} },
                    SensorDatapoint { wavelength: 610., rgb: Vector3 { x: 0.77, y: 0.08, z: 0.00} },
                    SensorDatapoint { wavelength: 620., rgb: Vector3 { x: 0.68, y: 0.04, z: 0.00} },
                    SensorDatapoint { wavelength: 630., rgb: Vector3 { x: 0.55, y: 0.03, z: 0.00} },
                    SensorDatapoint { wavelength: 640., rgb: Vector3 { x: 0.44, y: 0.02, z: 0.00} },
                    SensorDatapoint { wavelength: 650., rgb: Vector3 { x: 0.31, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 660., rgb: Vector3 { x: 0.21, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 670., rgb: Vector3 { x: 0.10, y: 0.01, z: 0.00} },
                    SensorDatapoint { wavelength: 680., rgb: Vector3 { x: 0.02, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 690., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 700., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 750., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 800., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 850., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                    SensorDatapoint { wavelength: 906., rgb: Vector3 { x: 0.00, y: 0.00, z: 0.00} },
                ],
            }
        )]
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

    sensor_index: usize,
    sensors: Vec<(String, Sensor)>,
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
            sensors,
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

                if ui.combo(
                    "select sensor",
                    &mut self.sensor_index,
                    self.sensors.as_slice(),
                    |(label, _lens)| std::borrow::Cow::Borrowed(label),
                ) {
                    queue.write_buffer(
                        &self.sensor_buffer,
                        0,
                        bytemuck::cast_slice(&self.sensors[self.sensor_index].1.get_data()),
                    );
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
