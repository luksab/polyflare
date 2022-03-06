use rayon::prelude::*;
use std::io::Write;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, read_to_string, DirBuilder},
    hash::{Hash, Hasher},
    iter,
    path::Path,
    time::Instant,
};

use directories::ProjectDirs;
use itertools::iproduct;
use polynomial_optics::{Polynom4d, Polynomial};
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, CommandEncoder, ComputePipeline, Device,
    Queue, RenderPipeline, SurfaceConfiguration, TextureFormat, TextureView,
};

use crate::{lens_state::LensState, texture::Texture};

#[allow(dead_code)]
struct GpuPolynomials {
    polynomials: Vec<Polynomial<f64, 4>>,
    pub polynomial_bind_group: BindGroup,
    pub polynomial_bind_group_layout: BindGroupLayout,
}

impl GpuPolynomials {
    /// reads from cache if found
    fn check_cache(
        num_dots: usize,
        num_terms: usize,
        degree: usize,
        lens_state: &LensState,
    ) -> Option<Vec<Polynomial<f64, 4>>> {
        let mut hasher = DefaultHasher::new();
        (num_dots, num_terms, degree, &lens_state.actual_lens).hash(&mut hasher);
        let hash = hasher.finish();

        let proj_dirs = ProjectDirs::from("de", "luksab", "polyflare").unwrap();
        let cache_dir = proj_dirs.config_dir().join(Path::new("poly_cache"));

        let path = cache_dir.join(format!("{:x}.poly", hash));
        if path.exists() {
            if let Ok(str) = std::fs::read_to_string(path) {
                let (num_dots_read, num_terms_read, degree_read, polynomials): (
                    usize,
                    usize,
                    usize,
                    _,
                ) = match ron::de::from_str(str.as_str()) {
                    Ok(lens) => lens,
                    Err(_) => return None,
                };
                if num_dots_read == num_dots && num_terms_read == num_terms && degree_read == degree
                {
                    return Some(polynomials);
                }
                return None;
            }
        }

        None
    }

    fn write_cache(
        num_dots: usize,
        num_terms: usize,
        degree: usize,
        lens_state: &LensState,
        polynomials: &Vec<Polynomial<f64, 4>>,
    ) {
        let mut hasher = DefaultHasher::new();
        (num_dots, num_terms, degree, &lens_state.actual_lens).hash(&mut hasher);
        let hash = hasher.finish();

        let proj_dirs = ProjectDirs::from("de", "luksab", "polyflare").unwrap();
        let dir = proj_dirs.config_dir().join(Path::new("poly_cache"));
        if !dir.is_dir() {
            println!("creating lens directory {:?}", &dir);
            DirBuilder::new().recursive(true).create(&dir).unwrap();
        }

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&dir.join(Path::new(&format!("{:x}.poly", hash))))
            .unwrap();
        let pretty_config = ron::ser::PrettyConfig::new();
        std::io::Write::write_all(
            &mut file,
            ron::ser::to_string_pretty(&(num_dots, num_terms, degree, polynomials), pretty_config)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        // handle errors
        file.sync_all().unwrap();
    }

    fn compute_polynomials(
        num_dots: usize,
        num_terms: usize,
        degree: usize,
        lens_state: &LensState,
    ) -> Vec<Polynomial<f64, 4>> {
        let now = Instant::now();

        let pos_params = lens_state.pos_params.clone();
        let lens = lens_state.actual_lens.clone();
        // for which_ghost in 1..2 {
        //     for dir_xy in 0..=0 {
        let polynomials = rayon::ThreadPoolBuilder::new()
            .num_threads(6)
            .build()
            .unwrap()
            .install(|| {
                iproduct!(
                    (1..lens_state.actual_lens.get_ghosts_indicies(1, 0).len()).into_iter(),
                    0..=1
                )
                .par_bridge()
                .map(|(which_ghost, dir_xy)| {
                    let lens = lens.clone();
                    let width = 1.;
                    let dots = &mut lens.get_dots_grid(
                        (num_dots as f64).powf(0.25) as u32,
                        cgmath::Vector3 {
                            x: 0.,
                            y: 0.,
                            z: pos_params[2] as f64,
                        },
                        which_ghost as u32,
                        pos_params[8] as f64,
                        [width as f64, width as f64],
                        true,
                    );
                    println!("dots: {}", dots.len());
                    if dir_xy == 0 {
                        let dots = &mut lens.get_dots_2dgrid(
                            (num_dots as f64).powf(0.5) as u32,
                            cgmath::Vector3 {
                                x: 0.,
                                y: 0.,
                                z: pos_params[2] as f64,
                            },
                            which_ghost as u32,
                            pos_params[8] as f64,
                            [width as f64, width as f64],
                            false,
                        );
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(format!("python/dots/dots,{}.csv", which_ghost))
                            .unwrap();
                        println!("dots: {}", dots.len());
                        // writeln!(file, "x, y, z, w, xo, yo").unwrap();
                        for dot in dots.iter() {
                            writeln!(
                                file,
                                "{}, {}, {}, {}",
                                dot.init_pos[2], dot.init_pos[3], dot.pos[0], dot.pos[1]
                            )
                            .unwrap();
                        }
                    }

                    let points = dots
                        .iter()
                        .map(|dot| {
                            (
                                dot.init_pos[0],
                                dot.init_pos[1],
                                dot.init_pos[2],
                                dot.init_pos[3],
                                // dot.strength,
                                dot.pos[dir_xy],
                            )
                        })
                        .collect::<Vec<_>>();
                    println!("points: {}", points.len());

                    let now = Instant::now();

                    // let mut file = std::fs::OpenOptions::new()
                    //     .write(true)
                    //     .create(true)
                    //     .truncate(true)
                    //     // .append(true)
                    //     .open("python/points.csv")
                    //     .unwrap();

                    // writeln!(file, "x, y, z, w, out").unwrap();
                    // for point in filtered_points.iter() {
                    //     writeln!(file, "{}, {}, {}, {}, {}", point.0, point.1, point.2, point.3, point.4).unwrap();
                    // }

                    // let basis =
                    //     polynomial_optics::LegendreBasis::new_from_grid(degree, num_dots, -1.0..1.);
                    // let basis = polynomial_optics::LegendreBasis::new(degree);

                    // for i in 0..polynomial_optics::Legendre4d::num_polys(degree) {
                    //     for j in 0..polynomial_optics::Legendre4d::num_polys(degree) {
                    //         let index =
                    //             polynomial_optics::Legendre4d::poly_index_to_multi_index(i, degree)
                    //                 .unwrap();
                    //         let index2 =
                    //             polynomial_optics::Legendre4d::poly_index_to_multi_index(j, degree)
                    //                 .unwrap();
                    //         let a = basis.integrate_4d(&filtered_points, index, index2);
                    //         println!("{:2} {:2} {:2.5}", i, j, a);
                    //     }
                    // }

                    // todo!();

                    // let mut legendre = polynomial_optics::Legendre4d::new(basis);
                    // legendre.fit(&filtered_points);

                    // let mut sum = 0.;
                    // for point in &filtered_points{
                    //     // println!("point: {:?}", point);
                    //     let eval = legendre.eval(&(point.0, point.1, point.2, point.3));
                    //     sum += (point.4 - eval).powi(2);
                    //     // println!("p: {}, l: {}, q: {}", point.4, eval, point.4 / eval);
                    // }
                    // println!("lg error: {}", (sum / filtered_points.len() as f64).sqrt());
                    // println!(
                    //     "lg error: {}",
                    //     legendre.approx_error(&filtered_points, 10000)
                    // );

                    // let mut sparse_legendre = legendre.get_sparse(num_terms);
                    // println!(
                    //     "ls error: {}",
                    //     sparse_legendre.approx_error(&filtered_points, 10000, 0)
                    // );

                    // sparse_legendre.fit(&filtered_points);
                    // println!(
                    //     "actual ls error: {}",
                    //     sparse_legendre.error(&filtered_points)
                    // );
                    // for samples in [1, 10, 100, 1000, 10000, 100000] {
                    //     let now = Instant::now();
                    //     let e = legendre.approx_error(&filtered_points, samples);
                    //     println!(
                    //         "{:6} lg error: {:1.6} in {}us",
                    //         samples,
                    //         e,
                    //         now.elapsed().as_micros()
                    //     );
                    // }

                    let polynom = dbg!(Polynom4d::<_>::fit(&points, degree));
                    println!("Fitting took {:?}", now.elapsed());

                    let mut sum = 0.;
                    for point in &points {
                        let eval = polynom.eval(point.0, point.1, point.2, point.3);
                        sum += (point.4 - eval).powi(2);
                        // println!("p: {}, l: {}, q: {}", point.4, eval, point.4 / eval);
                    }
                    println!("de error: {}", (sum / points.len() as f64).sqrt());

                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(format!("python/dots/depoly,{},{}.csv", which_ghost, dir_xy))
                        .unwrap();
                    // writeln!(file, "x, y, z, w, o").unwrap();
                    lens.get_dots_2dgrid(
                        (num_dots as f64).powf(0.5) as u32,
                        cgmath::Vector3 {
                            x: 0.,
                            y: 0.,
                            z: pos_params[2] as f64,
                        },
                        which_ghost as u32,
                        pos_params[8] as f64,
                        [width as f64, width as f64],
                        false,
                    )
                    .iter()
                    .map(|ray| (ray.init_pos[2], ray.init_pos[3], ray.pos[0].is_finite()))
                    .for_each(|(z, w, is_finite)| {
                        let (x, y) = (0., 0.);
                        if is_finite {
                            writeln!(file, "{}, {}, {}", z, w, polynom.eval(x, y, z, w)).unwrap();
                        } else {
                            writeln!(file, "{}, {}, {}", z, w, f32::NAN).unwrap();
                        }
                    });

                    if cfg!(config_assertions) {
                        println!("{}", polynom);
                    }
                    // let mut sparse_poly =
                    //     polynom.get_sparse(&filtered_points, num_terms, true, true);

                    // let mut sparse_poly = polynom.get_sparse_dumb(num_terms);

                    // return sparse_poly;

                    let sparse_poly = polynom.simulated_annealing(&points, num_terms, 1000, 1000);

                    println!("sparse_poly len: {}", sparse_poly.terms.len());

                    sum = 0.;
                    for point in &points {
                        let eval = sparse_poly.eval([point.0, point.1, point.2, point.3]);
                        sum += (point.4 - eval).powi(2);
                        // println!("p: {}, l: {}, q: {}", point.4, eval, point.4 / eval);
                    }
                    println!("sp error: {}", (sum / points.len() as f64).sqrt());

                    // gradient descent is just worse than fit
                    // sparse_poly.gradient_descent(&filtered_points, 100);
                    println!("actual error: {}", sparse_poly.error(&points));

                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(format!("python/dots/poly,{},{}.csv", which_ghost, dir_xy))
                        .unwrap();
                    // writeln!(file, "x, y, z, w, o").unwrap();
                    lens.get_dots_2dgrid(
                        (num_dots as f64).powf(0.5) as u32,
                        cgmath::Vector3 {
                            x: 0.,
                            y: 0.,
                            z: pos_params[2] as f64,
                        },
                        which_ghost as u32,
                        pos_params[8] as f64,
                        [width as f64, width as f64],
                        false,
                    )
                    .iter()
                    .map(|ray| (ray.init_pos[2], ray.init_pos[3], ray.pos[0].is_finite()))
                    .for_each(|(z, w, is_finite)| {
                        let (x, y) = (0., 0.);
                        if is_finite {
                            writeln!(file, "{}, {}, {}", z, w, sparse_poly.eval([x, y, z, w]))
                                .unwrap();
                        } else {
                            writeln!(file, "{}, {}, {}", z, w, f32::NAN).unwrap();
                        }
                    });

                    // panic!("{}", now.elapsed().as_millis());
                    sparse_poly
                })
                .collect::<Vec<_>>()
            });

        println!("Computing polynomials took {:?}", now.elapsed());
        // Arc::try_unwrap(polynomials).unwrap().into_inner().unwrap()
        polynomials
    }

    fn get_polynomials(
        num_dots: usize,
        num_terms: usize,
        degree: usize,
        lens_state: &LensState,
    ) -> Vec<Polynomial<f64, 4>> {
        Self::compute_polynomials(num_dots, num_terms, degree, lens_state);
        panic!("testing!");
        match Self::check_cache(num_dots, num_terms, degree, lens_state) {
            Some(polynomials) => polynomials,
            None => {
                let polynomials =
                    Self::compute_polynomials(num_dots, num_terms, degree, lens_state);
                Self::write_cache(num_dots, num_terms, degree, lens_state, &polynomials);
                polynomials
            }
        }
    }

    pub fn new(
        num_dots: usize,
        num_terms: usize,
        degree: usize,
        lens_state: &LensState,
        device: &Device,
    ) -> GpuPolynomials {
        let polynomials = Self::get_polynomials(num_dots, num_terms, degree, lens_state);

        let polynomial_bind_group_layout =
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
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("polynomial_bind_group_layout"),
            });

        let poly_data: Vec<f32> = polynomials
            .iter()
            .flat_map(|polynomial: &Polynomial<f64, 4>| polynomial.get_T_as_vec(num_terms))
            .map(|num| num as f32)
            .collect();

        // println!("poly_data: {:?}", poly_data);

        let polynomial_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&"Poly Buffer".to_string()),
            contents: bytemuck::cast_slice(&poly_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let poly_params = vec![num_terms as u32];
        let poly_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&"PolyParams Buffer".to_string()),
            contents: bytemuck::cast_slice(&poly_params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let polynomial_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &polynomial_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: polynomial_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: poly_params_buffer.as_entire_binding(),
                },
            ],
            label: Some("poly_bind_group"),
        });

        GpuPolynomials {
            polynomials,
            polynomial_bind_group,
            polynomial_bind_group_layout,
        }
    }
}

pub struct PolyPoly {
    tri_render_pipeline: wgpu::RenderPipeline,
    high_color_tex: Texture,
    conversion_render_pipeline: wgpu::RenderPipeline,
    conversion_bind_group: wgpu::BindGroup,
    compute_pipeline: ComputePipeline,
    compute_bind_group: BindGroup,
    compute_bind_group_layout: BindGroupLayout,
    pub vertex_buffer: wgpu::Buffer,
    tri_index_buffer: wgpu::Buffer,

    polynomials: GpuPolynomials,
    num_terms: usize,
    num_samples: usize,
    degree: usize,

    pub dot_side_len: u32,

    convert_meta: Option<std::fs::Metadata>,
    draw_meta: Option<std::fs::Metadata>,
    compute_meta: Option<std::fs::Metadata>,
    format: TextureFormat,
    conf_format: TextureFormat,
}

impl PolyPoly {
    fn shader_draw(
        device: &wgpu::Device,
        format: TextureFormat,
        params_bind_group_layout: &BindGroupLayout,
        lens_bind_group_layout: &BindGroupLayout,
    ) -> RenderPipeline {
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("polyPoly"),
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/lib/scenes/poly_poly/draw.wgsl")
                    .unwrap_or_else(|_| include_str!("draw.wgsl").to_string())
                    .into(),
            ),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[lens_bind_group_layout, params_bind_group_layout],
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "mainv",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // aperture position
                        wgpu::VertexAttribute {
                            offset: 2 * 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // entry aperture position
                        wgpu::VertexAttribute {
                            offset: 4 * 4,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // strength
                        wgpu::VertexAttribute {
                            offset: 6 * 4,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32,
                        },
                        // wavelength
                        wgpu::VertexAttribute {
                            offset: 7 * 4,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "mainf",
                targets: &[wgpu::ColorTargetState {
                    format,
                    // blend: None,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // topology: wgpu::PrimitiveTopology::PointList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    fn convert_shader(
        device: &wgpu::Device,
        params_bind_group_layout: &BindGroupLayout,
        polynomial_bind_group_layout: &BindGroupLayout,
        format: &TextureFormat,
        high_color_tex: &Texture,
    ) -> (RenderPipeline, BindGroup) {
        let conversion_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let conversion_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &conversion_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&high_color_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&high_color_tex.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });
        let conversion_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Conversion Pipeline Layout"),
                bind_group_layouts: &[
                    &conversion_bind_group_layout,
                    params_bind_group_layout,
                    polynomial_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let conversion_render_pipeline = {
            let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("conversion"),
                source: wgpu::ShaderSource::Wgsl(
                    read_to_string("gpu/src/lib/scenes/poly_poly/convert.wgsl")
                        .unwrap_or_else(|_| include_str!("convert.wgsl").to_string())
                        .into(),
                ),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&conversion_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "mainv",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 2 * 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "mainf",
                    targets: &[wgpu::ColorTargetState {
                        format: *format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                    unclipped_depth: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        (conversion_render_pipeline, conversion_bind_group)
    }

    fn raytrace_shader(
        device: &wgpu::Device,
        lens_bind_group_layout: &BindGroupLayout,
        params_bind_group_layout: &BindGroupLayout,
        polynomial_bind_group_layout: &BindGroupLayout,
        dot_side_len: u32,
        num_ghosts: u32,
    ) -> (ComputePipeline, BindGroup, BindGroupLayout, Buffer) {
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                read_to_string("gpu/src/lib/scenes/poly_poly/compute.wgsl")
                    .unwrap_or_else(|_| include_str!("compute.wgsl").to_string())
                    .into(),
            ),
        });

        // create compute bind layout group and compute pipeline layout
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[
                    &compute_bind_group_layout,
                    lens_bind_group_layout,
                    params_bind_group_layout,
                    polynomial_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // buffer for all particles data of type [bool,...]
        // vec3: 16 bytes, 4 floats
        // vec3, vec3, float
        let initial_ray_data =
            vec![0.1_f32; (dot_side_len * dot_side_len * num_ghosts * 8) as usize];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&"Rays Buffer".to_string()),
            contents: bytemuck::cast_slice(&initial_ray_data),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            }],
            label: None,
        });

        (
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            vertex_buffer,
        )
    }

    fn get_tri_index(device: &wgpu::Device, dot_side_len: u32, num_ghosts: u32) -> Buffer {
        // buffer for all verts data of type
        // vec2: 8 bytes, 2 floats
        // pos: vec2, aperture_pos: vec2, intensity: float + 1float alignment
        // let num_tris = (((dot_side_len - 1) * 2) * (dot_side_len - 1)) as usize;
        let mut initial_tri_index_data = vec![]; //0 as u32; (num_tris * 6) as usize];

        for i in 0..num_ghosts {
            let offset = i * (dot_side_len * dot_side_len);
            for y in 0..dot_side_len - 1 {
                for x in 0..dot_side_len - 1 {
                    // println!("x:{} y:{}", x, y);
                    initial_tri_index_data.push(x + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + 1 + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + dot_side_len + y * dot_side_len + offset);

                    initial_tri_index_data.push(x + 1 + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + dot_side_len + y * dot_side_len + offset);
                    initial_tri_index_data.push(x + 1 + dot_side_len + y * dot_side_len + offset);
                }
            }
        }

        if cfg!(debug_assertions) {
            println!(
                "initial_tri_index_data: {} {:?}",
                dot_side_len, initial_tri_index_data
            );
        }

        // assert_eq!(initial_tri_index_data.len() / 3, num_tris);

        // let now = Instant::now();
        // for i in 0..num_ghosts {
        //     initial_tri_index_data
        //         .append(&mut initial_tri_index_data.iter().map(|x| *x + offset).collect());
        // }
        // if cfg!(debug_assertions) {
        //     println!("Triangulate index buffer: {:?}", initial_tri_index_data);
        // }
        // println!("creating {} took {:?}", num_ghosts, now.elapsed());

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&"Tris Buffer".to_string()),
            contents: bytemuck::cast_slice(&initial_tri_index_data),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn get_num_tris(&self) -> u32 {
        (self.dot_side_len - 1) * (self.dot_side_len - 1)
    }

    pub async fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        lens_state: &LensState,
    ) -> Self {
        let num_terms = 100;
        let degree = 10;
        // let num_samples = 10 * {
        //     let mut num_terms = 0;
        //     for i in 0..=degree {
        //         for j in 0..=degree - i {
        //             for k in 0..=degree - (i + j) {
        //                 for _ in 0..=degree - (i + j + k) {
        //                     num_terms += 1;
        //                 }
        //             }
        //         }
        //     }
        //     num_terms
        // };
        let num_samples = 100_000;
        let polynomials = GpuPolynomials::new(num_samples, num_terms, degree, lens_state, device);

        let format = wgpu::TextureFormat::Rgba16Float;
        let high_color_tex =
            Texture::create_color_texture(device, config, format, "high_color_tex");

        let tri_render_pipeline = Self::shader_draw(
            device,
            format,
            &lens_state.params_bind_group_layout,
            &lens_state.lens_bind_group_layout,
        );

        let (conversion_render_pipeline, conversion_bind_group) = Self::convert_shader(
            device,
            &lens_state.params_bind_group_layout,
            &polynomials.polynomial_bind_group_layout,
            &config.format,
            &high_color_tex,
        );

        let dot_side_len = 2;
        let (compute_pipeline, compute_bind_group, compute_bind_group_layout, vertex_buffer) =
            Self::raytrace_shader(
                device,
                &lens_state.lens_bind_group_layout,
                &lens_state.params_bind_group_layout,
                &polynomials.polynomial_bind_group_layout,
                dot_side_len,
                lens_state.ghost_indices.len() as u32,
            );

        let tri_index_buffer =
            Self::get_tri_index(device, dot_side_len, lens_state.ghost_indices.len() as u32);

        let convert_meta = std::fs::metadata("gpu/src/lib/scenes/poly_poly/convert.wgsl").ok();
        let draw_meta = std::fs::metadata("gpu/src/lib/scenes/poly_poly/draw.wgsl").ok();
        let compute_meta = std::fs::metadata("gpu/src/lib/scenes/poly_poly/compute.wgsl").ok();

        Self {
            tri_render_pipeline,
            tri_index_buffer,
            polynomials,
            num_terms,
            num_samples,
            degree,

            vertex_buffer,
            high_color_tex,
            conversion_render_pipeline,
            conversion_bind_group,
            convert_meta,
            draw_meta,
            compute_meta,
            format,
            conf_format: config.format,
            dot_side_len,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
        }
    }

    pub fn get_dots(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        update_size: bool,
        lens_state: &LensState,
    ) -> Vec<polynomial_optics::DrawRay> {
        if update_size {
            // println!("update: {}", self.num_dots);
            let (compute_pipeline, compute_bind_group, compute_bind_group_layout, vertex_buffer) =
                Self::raytrace_shader(
                    device,
                    &lens_state.lens_bind_group_layout,
                    &lens_state.params_bind_group_layout,
                    &self.polynomials.polynomial_bind_group_layout,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
            self.compute_bind_group_layout = compute_bind_group_layout;
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.vertex_buffer = vertex_buffer;
            self.tri_index_buffer = Self::get_tri_index(
                device,
                self.dot_side_len,
                lens_state.ghost_indices.len() as u32,
            );
        }

        let num_ghosts = lens_state.ghost_indices.len() as u32;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Get dots Encoder"),
        });

        let work_group_count = std::cmp::min(
            (self.dot_side_len * self.dot_side_len * num_ghosts + 64 - 1) / 64,
            65535,
        ); // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.lens_bind_group, &[]);
            cpass.set_bind_group(2, &lens_state.params_bind_group, &[]);
            cpass.set_bind_group(3, &self.polynomials.polynomial_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }

        let output_buffer_size =
            (self.dot_side_len * self.dot_side_len * 32 * lens_state.ghost_indices.len() as u32)
                as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("Ray DST"),
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);
        encoder.copy_buffer_to_buffer(
            &self.vertex_buffer,
            0,
            &output_buffer,
            0,
            (self.dot_side_len * self.dot_side_len * 32 * lens_state.ghost_indices.len() as u32)
                .into(),
        );

        queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

        let mut out = vec![];
        device.poll(wgpu::Maintain::Wait);

        if let Ok(()) = pollster::block_on(buffer_future) {
            let data = buffer_slice.get_mapped_range();

            let vertices = unsafe { data.align_to::<f32>().1 };
            let vec_vertices = vertices.to_vec();
            let data = vec_vertices;

            for (i, elements) in data.chunks(8).enumerate() {
                let ray_num = i as u32 % (self.dot_side_len * self.dot_side_len);
                let ghost_num = i as u32 / (self.dot_side_len * self.dot_side_len);

                let ray_num_x =
                    (ray_num / (self.dot_side_len)) as f32 / (self.dot_side_len - 1) as f32;
                let ray_num_y =
                    (ray_num % (self.dot_side_len)) as f32 / (self.dot_side_len - 1) as f32;

                // print!("{:03}: ", i);
                let width = lens_state.pos_params[9];

                // println!(
                //     "ghost: {} num: {}, {:?}",
                //     ghost_num,
                //     ray_num,
                //     [
                //         ray_num_x as f32 * width - width / 2.,
                //         ray_num_y as f32 * width - width / 2.,
                //     ]
                // );
                out.push(polynomial_optics::DrawRay {
                    ghost_num,
                    init_pos: [
                        lens_state.pos_params[0] as f64,
                        lens_state.pos_params[1] as f64,
                        (ray_num_x as f32 * width - width / 2.) as f64,
                        (ray_num_y as f32 * width - width / 2.) as f64,
                    ],
                    pos: [elements[0] as _, elements[1] as _],
                    aperture_pos: [elements[2] as _, elements[3] as _],
                    entry_pos: [elements[4] as _, elements[5] as _],
                    strength: elements[6] as _,
                    wavelength: elements[7] as _,
                });
                // println!("{:?}", out.last().unwrap());
            }
            // println!("{:?}", data);
            out
        } else {
            panic!("Failed to copy ray buffer!")
        }
    }

    pub fn update_poly(&mut self, device: &wgpu::Device, lens_state: &LensState) {
        self.polynomials = GpuPolynomials::new(
            self.num_samples,
            self.num_terms,
            self.degree,
            lens_state,
            device,
        );
    }

    pub fn update_dots(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        update_size: bool,
        lens_state: &LensState,
    ) {
        if update_size {
            // println!("update: {}", self.num_dots);
            let (compute_pipeline, compute_bind_group, compute_bind_group_layout, vertex_buffer) =
                Self::raytrace_shader(
                    device,
                    &lens_state.lens_bind_group_layout,
                    &lens_state.params_bind_group_layout,
                    &self.polynomials.polynomial_bind_group_layout,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
            self.compute_bind_group_layout = compute_bind_group_layout;
            self.compute_pipeline = compute_pipeline;
            self.compute_bind_group = compute_bind_group;
            self.vertex_buffer = vertex_buffer;
            self.tri_index_buffer = Self::get_tri_index(
                device,
                self.dot_side_len,
                lens_state.ghost_indices.len() as u32,
            );
        }

        let num_ghosts = lens_state.ghost_indices.len() as u32;

        let work_group_count = std::cmp::min(
            (self.dot_side_len * self.dot_side_len * num_ghosts + 64 - 1) / 64,
            65535,
        ); // round up
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.set_bind_group(1, &lens_state.lens_bind_group, &[]);
            cpass.set_bind_group(2, &lens_state.params_bind_group, &[]);
            cpass.set_bind_group(3, &self.polynomials.polynomial_bind_group, &[]);
            cpass.dispatch(work_group_count, 1, 1);
        }
    }
}

impl PolyPoly {
    pub fn resize(
        &mut self,
        new_size: [u32; 2],
        scale_factor: f64,
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
    ) {
        // println!("size: {:?}, scale: {}", new_size.height * new_size.width * (scale_factor as u32) * (scale_factor as u32), scale_factor);
        let format = wgpu::TextureFormat::Rgba16Float;
        let mut config = config.clone();
        let scale_fact = 1.;
        config.width = (new_size[0] as f64 * scale_factor * scale_fact) as u32;
        config.height = (new_size[1] as f64 * scale_factor * scale_fact) as u32;

        self.high_color_tex =
            Texture::create_color_texture(device, &config, format, "high_color_tex");

        let conversion_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        self.conversion_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &conversion_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.high_color_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.high_color_tex.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });
    }

    pub fn update(&mut self, device: &wgpu::Device, lens_state: &LensState) {
        // if self.cell_timer.elapsed().unwrap().as_secs_f32() > 0.1 {
        //     self.cell_timer = SystemTime::now();
        //     self.update_cells(device, queue);
        // }
        //self.update_rays(device);
        if let Some(convert_meta) = &self.convert_meta {
            if convert_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/lib/scenes/poly_poly/convert.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                print!("reloading convert shader ");
                let now = Instant::now();
                self.convert_meta =
                    std::fs::metadata("gpu/src/lib/scenes/poly_poly/convert.wgsl").ok();
                let (pipeline, bind_group) = Self::convert_shader(
                    device,
                    &lens_state.params_bind_group_layout,
                    &self.polynomials.polynomial_bind_group_layout,
                    &self.conf_format,
                    &self.high_color_tex,
                );
                self.conversion_render_pipeline = pipeline;
                self.conversion_bind_group = bind_group;
                println!("took {:?}.", now.elapsed());
            }
        }
        if let Some(draw_meta) = &self.draw_meta {
            if draw_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/lib/scenes/poly_poly/draw.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                self.draw_meta = std::fs::metadata("gpu/src/lib/scenes/poly_poly/draw.wgsl").ok();
                print!("reloading draw shader ");
                let now = Instant::now();
                let pipeline = Self::shader_draw(
                    device,
                    self.format,
                    &lens_state.params_bind_group_layout,
                    &lens_state.lens_bind_group_layout,
                );
                self.tri_render_pipeline = pipeline;
                println!("took {:?}.", now.elapsed());
            }
        }
        if let Some(compute_meta) = &self.compute_meta {
            if compute_meta.modified().unwrap()
                != std::fs::metadata("gpu/src/lib/scenes/poly_poly/compute.wgsl")
                    .unwrap()
                    .modified()
                    .unwrap()
            {
                self.compute_meta =
                    std::fs::metadata("gpu/src/lib/scenes/poly_poly/compute.wgsl").ok();
                print!("reloading compute shader ");
                let now = Instant::now();
                let (pipeline, bind_group, compute_bind_group_layout, vertex_buffer) =
                    Self::raytrace_shader(
                        device,
                        &lens_state.lens_bind_group_layout,
                        &lens_state.params_bind_group_layout,
                        &self.polynomials.polynomial_bind_group_layout,
                        self.dot_side_len,
                        lens_state.ghost_indices.len() as u32,
                    );
                self.compute_bind_group_layout = compute_bind_group_layout;
                self.compute_pipeline = pipeline;
                self.compute_bind_group = bind_group;
                self.vertex_buffer = vertex_buffer;
                self.tri_index_buffer = Self::get_tri_index(
                    device,
                    self.dot_side_len,
                    lens_state.ghost_indices.len() as u32,
                );
                println!("took {:?}.", now.elapsed());
            }
        }
    }

    /// retrace rays with wavelengths and ghosts then render
    pub fn render_color(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        lens_state: &mut LensState,
        update: bool,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut first = true;
        for wavelen in 0..lens_state.num_wavelengths {
            let start_wavelen = 0.38;
            let end_wavelen = 0.78;
            let wavelength = start_wavelen
                + wavelen as f64
                    * ((end_wavelen - start_wavelen) / lens_state.num_wavelengths as f64);
            // let strength = polynomial_optics::Lens::str_from_wavelen(wavelength) / 10.;

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            lens_state.pos_params[3] = wavelength as f32;
            queue.write_buffer(
                &lens_state.pos_params_buffer,
                0,
                bytemuck::cast_slice(&lens_state.pos_params),
            );
            // lens_state.pos_params[7] = strength as f32;

            self.update_dots(device, &mut encoder, update, lens_state);

            self.render_dots(
                &self.high_color_tex.view,
                &mut encoder,
                lens_state,
                first,
                lens_state.ghost_indices.len() as u32,
            );
            queue.submit(iter::once(encoder.finish()));
            first = false;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        if cfg!(debug_assertions) {
            let output_buffer_size = (self.dot_side_len
                * self.dot_side_len
                * 24
                * lens_state.ghost_indices.len() as u32)
                as wgpu::BufferAddress;
            let output_buffer_desc = wgpu::BufferDescriptor {
                size: output_buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                label: Some("Ray DST"),
                mapped_at_creation: false,
            };
            let output_buffer = device.create_buffer(&output_buffer_desc);
            encoder.copy_buffer_to_buffer(
                &self.vertex_buffer,
                0,
                &output_buffer,
                0,
                (self.dot_side_len
                    * self.dot_side_len
                    * 24
                    * lens_state.ghost_indices.len() as u32)
                    .into(),
            );

            self.convert(device, &mut encoder, view, lens_state);

            queue.submit(Some(encoder.finish()));

            let buffer_slice = output_buffer.slice(..);
            let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
            device.poll(wgpu::Maintain::Wait);

            if let Ok(()) = pollster::block_on(buffer_future) {
                let data = buffer_slice.get_mapped_range();

                let vertices = unsafe { data.align_to::<f32>().1 };
                let vec_vertices = vertices.to_vec();
                let data = vec_vertices;

                println!("----------------------------------------------------------------------------------");
                for (i, elements) in data.chunks(6).enumerate() {
                    print!("{:03}:", i);
                    print!("pos: {}, {}  ", elements[0], elements[1]);
                    print!("aper: {}, {}  ", elements[2], elements[3]);
                    print!("s: {}", elements[4]);
                    println!("w: {}", elements[5]);
                }
                // println!("{:?}", data);
            } else {
                panic!("Failed to copy ray buffer!")
            }
        } else {
            self.convert(device, &mut encoder, view, lens_state);
            queue.submit(iter::once(encoder.finish()));
        }

        Ok(())
    }

    /// retrace rays with wavelengths and ghosts then render
    pub fn render_hires(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        lens_state: &mut LensState,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut first = true;
        // let opacity = lens_state.opacity;
        // lens_state.opacity /= lens_state.num_wavelengths as f32;

        // lens_state.update(device, queue);
        for wavelen in 0..lens_state.num_wavelengths {
            let start_wavelen = 0.38;
            let end_wavelen = 0.78;
            let wavelength = start_wavelen
                + wavelen as f64
                    * ((end_wavelen - start_wavelen) / lens_state.num_wavelengths as f64);
            // let strength = polynomial_optics::Lens::str_from_wavelen(wavelength) / 10.;

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            lens_state.pos_params[3] = wavelength as f32;
            queue.write_buffer(
                &lens_state.pos_params_buffer,
                0,
                bytemuck::cast_slice(&lens_state.pos_params),
            );
            // lens_state.pos_params[7] = strength as f32;

            self.update_dots(device, &mut encoder, true, lens_state);

            self.render_dots(
                &self.high_color_tex.view,
                &mut encoder,
                lens_state,
                first,
                lens_state.ghost_indices.len() as u32,
            );
            queue.submit(iter::once(encoder.finish()));
            device.poll(wgpu::Maintain::Wait);
            first = false;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.convert(device, &mut encoder, view, lens_state);
        queue.submit(iter::once(encoder.finish()));

        // lens_state.opacity = opacity;
        // lens_state.update(device, queue);
        Ok(())
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        device: &wgpu::Device,
        queue: &Queue,
        lens_state: &LensState,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.render_dots(&self.high_color_tex.view, &mut encoder, lens_state, true, 1);

        self.convert(device, &mut encoder, view, lens_state);

        queue.submit(iter::once(encoder.finish()));

        // conversion pass

        Ok(())
    }

    pub fn render_dots(
        &self,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        lens_state: &LensState,
        clear: bool,
        num_ghosts: u32,
    ) {
        // create render pass descriptor and its color attachments
        let color_attachments = [wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: if clear {
                    wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    })
                } else {
                    wgpu::LoadOp::Load
                },
                store: true,
            },
        }];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
        };

        // println!("{},{},{}", rays[3], rays[4], rays[5]);

        //let rays = vec![-1.0, -1.0, 0.0, 0.0, 1.0, 1.0];
        {
            // render pass
            let mut rpass = encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.tri_render_pipeline);
            rpass.set_bind_group(0, &lens_state.lens_bind_group, &[]);
            rpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            // the three instance-local vertices
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.tri_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw_indexed(0..self.get_num_tris() * num_ghosts * 6, 0, 0..1);
            // rpass.draw(0..self.dot_side_len * self.dot_side_len, 0..1);
        }
    }

    pub fn convert(
        &self,
        device: &wgpu::Device,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        lens_state: &LensState,
    ) {
        let vertex_buffer_data = [
            -1.0f32, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0,
        ];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&vertex_buffer_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // create render pass descriptor and its color attachments
        let color_attachments = [wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: true,
            },
        }];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
        };

        {
            // render pass
            let mut rpass = encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.conversion_render_pipeline);
            rpass.set_bind_group(0, &self.conversion_bind_group, &[]);
            rpass.set_bind_group(1, &lens_state.params_bind_group, &[]);
            rpass.set_bind_group(2, &self.polynomials.polynomial_bind_group, &[]);
            rpass.set_vertex_buffer(0, vertices_buffer.slice(..));
            //render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            rpass.draw(0..vertex_buffer_data.len() as u32 / 2, 0..1);
        }
    }
}
