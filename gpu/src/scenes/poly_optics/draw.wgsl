struct VertexInput {
    [[location(0)]] o: vec3<f32>;
    [[location(1)]] wavelength: f32;
    [[location(2)]] d: vec3<f32>;
    [[location(3)]] strength: f32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] strength: f32;
    [[location(1)]] wavelength: f32;
};

struct SimParams {
  opacity: f32;
  width_scaled: f32;
  height_scaled: f32;
  width: f32;
  height: f32;
  draw_mode: f32;
  which_ghost: f32;
  window_width_scaled: f32;
  window_height_scaled: f32;
  window_width: f32;
  window_height: f32;
  side_len: f32;
  zoom: f32;
};

struct SensorDatapoint {
    rgb: vec3<f32>;
    wavelength: f32;
};

struct Sensor {
    measuremens: [[stride(16)]] array<SensorDatapoint>;
};

[[group(0), binding(2)]] var<uniform> params : SimParams;
[[group(0), binding(1)]] var<storage, read> sensor : Sensor;

fn lookup_rgb(wavelength: f32) -> vec3<f32> {
    let lower_index = u32(clamp((wavelength - sensor.measuremens[0].wavelength / 1000.) * 100., 0., 34.));
    let factor = (wavelength % 0.1) * 10.;
    return sensor.measuremens[lower_index].rgb * (1. - factor)
     + sensor.measuremens[lower_index + u32(1)].rgb * (factor);
}

[[stage(vertex)]]
fn mainv(
    in: VertexInput,
) -> VertexOutput {
    let screenAspect = normalize(vec2<f32>(params.height_scaled, params.width_scaled));

    var out: VertexOutput;
    out.clip_position = vec4<f32>(vec2<f32>(in.o.z, in.o.y) / 4.0 * screenAspect, 0.,1.);
    out.strength = in.strength;
    out.wavelength = in.wavelength;
    return out;
}

[[stage(fragment)]]
fn mainf(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let s = in.strength * params.opacity;
  var rgb = lookup_rgb(in.wavelength);
  rgb.g = rgb.g * 0.6;
  return vec4<f32>(rgb, sqrt(in.strength) * params.opacity);
  // return vec4<f32>(1.0, 1.0, 1.0, 0.0);
}
