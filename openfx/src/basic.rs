use ofx::*;
use std::{sync::{Arc, Mutex, RwLock}, time::Duration, thread};

use crate::wgpu_ofx::Gpu;

plugin_module!(
    "de.luksab.polyflare.basic",
    ApiVersion(1),
    PluginVersion(1, 0),
    SimplePlugin::new
);

struct SimplePlugin {
    host_supports_multiple_clip_depths: Bool,
    gpu: Arc<Mutex<Gpu>>,
}

impl SimplePlugin {
    pub fn new() -> SimplePlugin {
        let mut gpu = Gpu::new();
        // gpu.update(true);
        SimplePlugin {
            gpu: Arc::new(Mutex::new(gpu)),
            host_supports_multiple_clip_depths: false,
        }
    }
}
#[allow(unused)]
struct MyInstanceData {
    is_general_effect: bool,

    source_clip: ClipInstance,
    // mask_clip: Option<ClipInstance>,
    output_clip: ClipInstance,

    dots_exponent: ParamHandle<Double>,
    opacity: ParamHandle<Double>,
    zoom_fact: ParamHandle<Double>,
    scale_fact: ParamHandle<Double>,
    num_wavelengths: ParamHandle<Double>,
    draw_mode: ParamHandle<Double>,
    pos_x_param: ParamHandle<Double>,
    pos_y_param: ParamHandle<Double>,
    pos_z_param: ParamHandle<Double>,
    lens_param: ParamHandle<Double>,
    entry_param: ParamHandle<Double>,
    width_param: ParamHandle<Double>,
    // gpu: Arc<Mutex<Gpu>>,
}

struct TileProcessor<'a> {
    instance: ImageEffectHandle,
    src: ImageDescriptor<'a, RGBAColourF>,
    raw: Arc<RwLock<Vec<Vec<(f32, f32, f32, f32)>>>>,
    dst: ImageTileMut<'a, RGBAColourF>,
    render_window: RectI,
}

// Members of the TileProcessor are either:
// - shared + read only
// - shareable handles and memory blocks from OFX (for which we can spray and pray)
// - owned by the tileprocessor
// so we can assume it can be processed across multiple threads even if rustc says no
//
unsafe impl<'a> Send for TileProcessor<'a> {}

impl<'a> TileProcessor<'a> {
    fn new(
        instance: ImageEffectHandle,
        src: ImageDescriptor<'a, RGBAColourF>,
        raw: Arc<RwLock<Vec<Vec<(f32, f32, f32, f32)>>>>,
        dst: ImageTileMut<'a, RGBAColourF>,
        render_window: RectI,
    ) -> Self {
        TileProcessor {
            instance,
            src,
            raw,
            dst,
            // mask,
            render_window,
        }
    }
}

struct TileDispatch<'a> {
    tiles: Arc<Mutex<Vec<TileProcessor<'a>>>>,
}

impl<'a> TileDispatch<'a> {
    fn new(tiles: Vec<TileProcessor<'a>>) -> Self {
        TileDispatch {
            tiles: Arc::new(Mutex::new(tiles)),
        }
    }

    fn pop(&mut self) -> Option<TileProcessor<'a>> {
        if let Ok(ref mut tiles) = self.tiles.lock() {
            tiles.pop()
        } else {
            None
        }
    }
}

impl<'a> Runnable for TileDispatch<'a> {
    fn run(&mut self, _thread_index: UnsignedInt, _thread_max: UnsignedInt) {
        while let Some(mut tile) = self.pop() {
            if tile.do_processing().is_err() {
                break;
            };
        }
    }
}

trait ProcessRGBA<'a> {
    fn do_processing(&'a mut self) -> Result<()>;
}

impl<'a> ProcessRGBA<'a> for TileProcessor<'a> {
    fn do_processing(&'a mut self) -> Result<()> {
        let proc_window = self.render_window;

        let raw = self.raw.write().unwrap();
        for y in self.dst.y1.max(proc_window.y1)..self.dst.y2.min(proc_window.y2) {
            let dst_row = self.dst.row_range(proc_window.x1, proc_window.x2, y);
            let src_row = self.src.row_range(proc_window.x1, proc_window.x2, y);

            if self.instance.abort()? {
                break;
            }

            for (x, (dst, src)) in dst_row.iter_mut().zip(src_row.iter()).enumerate() {
                if y as usize >= raw.len() || x >= raw[0].len() {
                    continue; // should be out of index!
                }
                *dst.channel_mut(0) = raw[y as usize][x].0 + src.r();
                *dst.channel_mut(1) = raw[y as usize][x].1 + src.g();
                *dst.channel_mut(2) = raw[y as usize][x].2 + src.b();

                *dst.channel_mut(3) = 1.; // set alpha to 1.0
            }
        }

        Ok(())
    }
}

const PARAM_MAIN_NAME: &str = "Main";
const PARAM_OPACITY_NAME: &str = "opacity";
const PARAM_DOTS_NAME: &str = "dots";
const PARAM_ZOOM_NAME: &str = "zoom";
const PARAM_SCALE_NAME: &str = "scale";
const PARAM_WAVELEN_NAME: &str = "wavelenghts";
const PARAM_MODE_NAME: &str = "draw_mode";
const PARAM_X_NAME: &str = "x";
const PARAM_Y_NAME: &str = "y";
const PARAM_Z_NAME: &str = "z";
const PARAM_LENS_NAME: &str = "lens";
const PARAM_ENTRY_NAME: &str = "entry";
const PARAM_WIDTH_NAME: &str = "width";

impl Execute for SimplePlugin {
    #[allow(clippy::float_cmp)]
    fn execute(&mut self, plugin_context: &PluginContext, action: &mut Action) -> Result<Int> {
        let mut gpu = self.gpu.lock().unwrap();
        use Action::*;
        match *action {
            Render(ref mut effect, ref in_args) => {
                println!("render");
                let time = in_args.get_time()?;
                // TODO: what happens if render_window < full size?
                let render_window = in_args.get_render_window()?;
                let instance_data: &mut MyInstanceData = effect.get_instance_data()?;

                let source_image = instance_data.source_clip.get_image(time)?;
                let output_image = instance_data.output_clip.get_image_mut(time)?;
                // let mask_image = match instance_data.mask_clip {
                //     None => None,
                //     Some(ref mask_clip) => {
                //         if instance_data.is_general_effect && mask_clip.get_connected()? {
                //             Some(mask_clip.get_image(time)?)
                //         } else {
                //             None
                //         }
                //     }
                // };

                let mut output_image = output_image.borrow_mut();
                let num_threads = plugin_context.num_threads()?;
                let num_tiles = num_threads as usize;

                match (
                    output_image.get_pixel_depth()?,
                    output_image.get_components()?,
                ) {
                    (BitDepth::Float, ImageComponent::RGBA) => {
                        let size = output_image.get_region_of_definition()?;
                        let size = ((size.x2 - size.x1) as u32, (size.y2 - size.y1) as u32);
                        gpu.resize([size.0, size.1]);
                        gpu.update(instance_data.get_data(time));
                        let raw = Arc::new(RwLock::new(gpu.render()));

                        // TODO: make sure no other thread gets a write lock
                        // before this read lock is aquired

                        let mut queue = TileDispatch::new(
                            output_image
                                .get_tiles_mut::<RGBAColourF>(num_tiles)?
                                .into_iter()
                                .map(|tile| {
                                    let src = source_image.get_descriptor::<RGBAColourF>().unwrap();
                                    // let mask = mask_image
                                    //     .as_ref()
                                    //     .and_then(|mask| mask.get_descriptor::<f32>().ok());
                                    TileProcessor::new(
                                        effect.clone(),
                                        src,
                                        raw.clone(),
                                        tile,
                                        render_window,
                                    )
                                })
                                .collect(),
                        );
                        plugin_context.run_in_threads(num_threads, &mut queue)?;
                        drop(gpu);
                    }
                    (_, _) => return FAILED,
                }

                if effect.abort()? {
                    FAILED
                } else {
                    OK
                }
            }

            IsIdentity(ref mut effect, ref in_args, ref mut out_args) => {
                // let time = in_args.get_time()?;
                // let _render_window = in_args.get_render_window()?;
                // let instance_data: &MyInstanceData = effect.get_instance_data()?;

                // let (scale_value, sr, sg, sb, sa) = instance_data.get_scale_components(time)?;

                // if scale_value == 1. && sr == 1. && sg == 1. && sb == 1. && sa == 1. {
                //     out_args.set_name(&image_effect_simple_source_clip_name())?;
                //     OK
                // } else {
                //     REPLY_DEFAULT
                // }
                REPLY_DEFAULT
            }

            InstanceChanged(ref mut effect, ref in_args) => {
                // if in_args.get_change_reason()? == Change::UserEdited {
                let obj_changed = in_args.get_name()?;
                let instance_data: &mut MyInstanceData = effect.get_instance_data()?;
                let time = in_args.get_time()?;
                println!("Instance changed: {}", obj_changed);
                // let mut gpu = self.gpu.lock().unwrap();
                gpu.update(instance_data.get_data(time));

                OK
            }

            GetRegionOfDefinition(ref mut effect, ref in_args, ref mut out_args) => {
                let time = in_args.get_time()?;
                let rod = effect
                    .get_instance_data::<MyInstanceData>()?
                    .source_clip
                    .get_region_of_definition(time)?;
                out_args.set_effect_region_of_definition(rod)?;

                OK
            }

            GetRegionsOfInterest(ref mut effect, ref in_args, ref mut out_args) => {
                let roi = in_args.get_region_of_interest()?;

                out_args.set_raw(image_clip_prop_roi!(clip_source!()), &roi)?;

                // if effect
                //     .get_instance_data::<MyInstanceData>()?
                //     .is_general_effect
                //     && effect.get_clip(clip_mask!())?.get_connected()?
                // {
                //     out_args.set_raw(image_clip_prop_roi!(clip_mask!()), &roi)?;
                // }

                OK
            }

            GetTimeDomain(ref mut effect, ref mut out_args) => {
                let my_data: &MyInstanceData = effect.get_instance_data()?;
                out_args.set_frame_range(my_data.source_clip.get_frame_range()?)?;

                OK
            }

            GetClipPreferences(ref mut effect, ref mut out_args) => {
                let my_data: &MyInstanceData = effect.get_instance_data()?;
                let bit_depth = my_data.source_clip.get_pixel_depth()?;
                let image_component = my_data.source_clip.get_components()?;
                let output_component = match image_component {
                    ImageComponent::RGBA | ImageComponent::RGB => ImageComponent::RGBA,
                    _ => ImageComponent::Alpha,
                };
                out_args.set_raw(
                    image_clip_prop_components!(clip_output!()),
                    output_component.to_bytes(),
                )?;

                if self.host_supports_multiple_clip_depths {
                    out_args
                        .set_raw(image_clip_prop_depth!(clip_output!()), bit_depth.to_bytes())?;
                }

                // if my_data.is_general_effect {
                //     let is_mask_connected = my_data
                //         .mask_clip
                //         .as_ref()
                //         .and_then(|mask| mask.get_connected().ok())
                //         .unwrap_or_default();

                //     if is_mask_connected {
                //         out_args.set_raw(
                //             image_clip_prop_components!(clip_mask!()),
                //             ImageComponent::Alpha.to_bytes(),
                //         )?;
                //         if self.host_supports_multiple_clip_depths {
                //             out_args.set_raw(
                //                 image_clip_prop_depth!(clip_mask!()),
                //                 bit_depth.to_bytes(),
                //             )?;
                //         }
                //     }
                // }

                OK
            }

            CreateInstance(ref mut effect) => {
                println!("CreateInstance");
                // let gpu = self.gpu.lock().unwrap();
                let mut effect_props: EffectInstance = effect.properties()?;
                let mut param_set = effect.parameter_set()?;

                let is_general_effect = effect_props.get_context()?.is_general();
                // let per_component_scale_param = param_set.parameter(PARAM_SCALE_COMPONENTS_NAME)?;

                let source_clip = effect.get_simple_input_clip()?;
                let output_clip = effect.get_output_clip()?;
                // let mask_clip = if is_general_effect {
                //     Some(effect.get_clip(clip_mask!())?)
                // } else {
                //     None
                // };

                let dots_exponent = param_set.parameter(PARAM_DOTS_NAME)?;
                let zoom_fact = param_set.parameter(PARAM_ZOOM_NAME)?;
                let scale_fact = param_set.parameter(PARAM_SCALE_NAME)?;
                let opacity = param_set.parameter(PARAM_OPACITY_NAME)?;
                let num_wavelengths = param_set.parameter(PARAM_WAVELEN_NAME)?;
                let draw_mode = param_set.parameter(PARAM_MODE_NAME)?;
                let pos_x_param = param_set.parameter(PARAM_X_NAME)?;
                let pos_y_param = param_set.parameter(PARAM_Y_NAME)?;
                let pos_z_param = param_set.parameter(PARAM_Z_NAME)?;
                let lens_param = param_set.parameter(PARAM_LENS_NAME)?;
                let entry_param = param_set.parameter(PARAM_ENTRY_NAME)?;
                let width_param = param_set.parameter(PARAM_WIDTH_NAME)?;

                let data = MyInstanceData {
                    is_general_effect,
                    source_clip,
                    // mask_clip,
                    output_clip,
                    dots_exponent,
                    opacity,
                    zoom_fact,
                    scale_fact,
                    num_wavelengths,
                    draw_mode,
                    pos_x_param,
                    pos_y_param,
                    pos_z_param,
                    lens_param,
                    entry_param,
                    width_param,
                    // gpu: self.gpu.clone(),
                };
                // let mut gpu = self.gpu.lock().unwrap();
                gpu.update(data.get_data(1.0));
                effect.set_instance_data(data)?;

                // Self::set_per_component_scale_enabledness(effect)?;

                OK
            }

            DestroyInstance(ref mut _effect) => {
                println!("DestroyInstance");
                OK
            }

            DescribeInContext(ref mut effect, ref in_args) => {
                let mut output_clip = effect.new_output_clip()?;
                output_clip
                    .set_supported_components(&[ImageComponent::RGBA, ImageComponent::Alpha])?;

                let mut input_clip = effect.new_simple_input_clip()?;
                input_clip
                    .set_supported_components(&[ImageComponent::RGBA, ImageComponent::Alpha])?;

                // if in_args.get_context()?.is_general() {
                //     let mut mask = effect.new_clip(clip_mask!())?;
                //     mask.set_supported_components(&[ImageComponent::Alpha])?;
                //     mask.set_optional(true)?;
                // }

                fn define_scale_param(
                    param_set: &mut ParamSetHandle,
                    name: &str,
                    label: &'static str,
                    script_name: &'static str,
                    hint: &'static str,
                    parent: Option<&'static str>,
                    min: f64,
                    max: f64,
                ) -> Result<()> {
                    let mut param_props = param_set.param_define_double(name)?;

                    param_props.set_double_type(ParamDoubleType::Scale)?;
                    param_props.set_label(label)?;
                    param_props.set_default(1.0)?;
                    param_props.set_display_min(min)?;
                    param_props.set_display_max(max)?;
                    param_props.set_hint(hint)?;
                    param_props.set_script_name(script_name)?;

                    if let Some(parent) = parent {
                        param_props.set_parent(parent)?;
                    }

                    Ok(())
                }

                /*
                dots_exponent: ParamHandle<Double>,
                opacity: ParamHandle<Double>,
                scale_fact: ParamHandle<Double>,
                num_wavelengths: ParamHandle<Double>,
                triangulate: bool,
                pos_x_param: ParamHandle<Double>,
                pos_y_param: ParamHandle<Double>,
                pos_z_param: ParamHandle<Double>,
                */

                let mut param_set = effect.parameter_set()?;
                define_scale_param(
                    &mut param_set,
                    PARAM_OPACITY_NAME,
                    PARAM_OPACITY_NAME,
                    PARAM_OPACITY_NAME,
                    "Opacity of the flare",
                    None,
                    0.1,
                    10.0,
                )?;

                let mut param_set = effect.parameter_set()?;
                define_scale_param(
                    &mut param_set,
                    PARAM_DOTS_NAME,
                    PARAM_DOTS_NAME,
                    PARAM_DOTS_NAME,
                    "Exponent for the dots",
                    None,
                    1.,
                    5.,
                )?;

                define_scale_param(
                    &mut param_set,
                    PARAM_ZOOM_NAME,
                    PARAM_ZOOM_NAME,
                    PARAM_ZOOM_NAME,
                    "Scales the image",
                    None,
                    0.1,
                    4.0,
                )?;

                define_scale_param(
                    &mut param_set,
                    PARAM_SCALE_NAME,
                    PARAM_SCALE_NAME,
                    PARAM_SCALE_NAME,
                    "Oversampling factor",
                    None,
                    0.1,
                    10.0,
                )?;

                define_scale_param(
                    &mut param_set,
                    PARAM_WAVELEN_NAME,
                    PARAM_WAVELEN_NAME,
                    PARAM_WAVELEN_NAME,
                    "Number of wavelengths",
                    None,
                    1.,
                    50.,
                )?;

                define_scale_param(
                    &mut param_set,
                    PARAM_MODE_NAME,
                    PARAM_MODE_NAME,
                    PARAM_MODE_NAME,
                    "What mode to draw with: 0: dense, 1: sparse, 2: poly",
                    None,
                    0.,
                    2.9,
                )?;

                // let mut param_props = param_set.param_define_group(PARAM_COMPONENT_SCALES_NAME)?;
                // param_props.set_hint("Scales on the individual component")?;
                // param_props.set_label("Components")?;

                define_scale_param(
                    &mut param_set,
                    PARAM_X_NAME,
                    "X",
                    PARAM_X_NAME,
                    "X component of the origin of the flare",
                    Some(PARAM_X_NAME),
                    -3.,
                    3.,
                )?;
                define_scale_param(
                    &mut param_set,
                    PARAM_Y_NAME,
                    "Y",
                    PARAM_Y_NAME,
                    "Y component of the origin of the flare",
                    Some(PARAM_Y_NAME),
                    -3.,
                    3.,
                )?;
                define_scale_param(
                    &mut param_set,
                    PARAM_Z_NAME,
                    "Z",
                    PARAM_Z_NAME,
                    "Z component of the origin of the flare",
                    Some(PARAM_Z_NAME),
                    -10.,
                    0.,
                )?;

                println!("Current Lens: {}", gpu.lens_ui.current_filename.as_str());
                define_scale_param(
                    &mut param_set,
                    PARAM_LENS_NAME,
                    PARAM_LENS_NAME,
                    PARAM_LENS_NAME,
                    "Which Lens to render",
                    None,
                    0.,
                    gpu::lens_state::LensState::get_lenses().len() as f64 - 1.,
                )?;

                define_scale_param(
                    &mut param_set,
                    PARAM_ENTRY_NAME,
                    PARAM_ENTRY_NAME,
                    PARAM_ENTRY_NAME,
                    "Radius of the entry aperture",
                    None,
                    0.,
                    5.,
                )?;

                define_scale_param(
                    &mut param_set,
                    PARAM_WIDTH_NAME,
                    PARAM_WIDTH_NAME,
                    PARAM_WIDTH_NAME,
                    "Width of sampled direction",
                    None,
                    0.,
                    5.,
                )?;

                param_set
                    .param_define_page(PARAM_MAIN_NAME)?
                    .set_children(&[
                        PARAM_OPACITY_NAME,
                        PARAM_DOTS_NAME,
                        PARAM_ZOOM_NAME,
                        PARAM_SCALE_NAME,
                        PARAM_WAVELEN_NAME,
                        PARAM_X_NAME,
                        PARAM_Y_NAME,
                        PARAM_Z_NAME,
                        PARAM_MODE_NAME,
                        PARAM_LENS_NAME,
                        PARAM_ENTRY_NAME,
                        PARAM_WIDTH_NAME,
                    ])?;
                OK
            }

            Describe(ref mut effect) => {
                self.host_supports_multiple_clip_depths = plugin_context
                    .get_host()
                    .get_supports_multiple_clip_depths()?;

                let mut effect_properties: EffectDescriptor = effect.properties()?;
                effect_properties.set_grouping("Flares")?;

                effect_properties.set_label("Polyflare")?;
                effect_properties.set_short_label("Polyflare")?;
                effect_properties.set_long_label("Ofx-rs Polyflare")?;

                effect_properties.set_supported_pixel_depths(&[BitDepth::Float])?;
                effect_properties.set_supported_contexts(&[
                    // ImageEffectContext::Filter,
                    ImageEffectContext::General,
                ])?;

                OK
            }

            _ => REPLY_DEFAULT,
        }
    }
}

// impl SimplePlugin {
//     fn set_per_component_scale_enabledness(effect: &mut ImageEffectHandle) -> Result<()> {
//         let instance_data: &mut MyInstanceData = effect.get_instance_data()?;
//         let input_clip = effect.get_simple_input_clip()?;
//         let is_input_rgb = input_clip.get_connected()? && input_clip.get_components()?.is_rgb();
//         instance_data
//             .per_component_scale_param
//             .set_enabled(is_input_rgb)?;
//         let per_component_scale =
//             is_input_rgb && instance_data.per_component_scale_param.get_value()?;
//         for scale_param in &mut [
//             &mut instance_data.scale_r_param,
//             &mut instance_data.scale_g_param,
//             &mut instance_data.scale_b_param,
//             &mut instance_data.scale_a_param,
//         ] {
//             scale_param.set_enabled(per_component_scale)?;
//             instance_data
//                 .scale_param
//                 .set_enabled(!per_component_scale)?
//         }

//         Ok(())
//     }
// }

impl MyInstanceData {
    fn get_data(&self, time: Time) -> Result<(f64, f64, f64, f64, f64, usize, f64, f64, f64, usize, f64, f64)> {
        Ok((
            self.dots_exponent.get_value_at_time(time)?,
            self.num_wavelengths.get_value_at_time(time)?,
            self.opacity.get_value_at_time(time)?,
            self.zoom_fact.get_value_at_time(time)?,
            self.scale_fact.get_value_at_time(time)?,
            self.draw_mode.get_value_at_time(time)? as usize,
            self.pos_x_param.get_value_at_time(time)?,
            self.pos_y_param.get_value_at_time(time)?,
            self.pos_z_param.get_value_at_time(time)?,
            self.lens_param.get_value_at_time(time)? as usize,
            self.entry_param.get_value_at_time(time)?,
            self.width_param.get_value_at_time(time)?,
        ))
    }
}
