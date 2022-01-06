struct VertexInput {
    [[location(0)]] pos: vec2<f32>;
    [[location(1)]] aperture_pos: vec2<f32>;
    [[location(2)]] entry_pos: vec2<f32>;
    [[location(3)]] strength: f32;
    [[location(4)]] wavelength: f32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] strength: f32;
    [[location(1)]] rgb: vec3<f32>;
    [[location(2)]] aperture_pos: vec2<f32>;
    [[location(3)]] entry_pos: vec2<f32>;
};

/// one Lens Element 
/// - one optical interface between glass and air
struct Element {
  radius: f32;
  b1: f32;
  b2: f32;
  b3: f32;
  c1: f32;
  c2: f32;
  c3: f32;
  b1_2: f32;
  b2_2: f32;
  b3_2: f32;
  c1_2: f32;
  c2_2: f32;
  c3_2: f32;
  coating_ior: f32;
  coating_thickness: f32;
  position: f32;// num_blades if aperture
  entry: f32;// 0: false, 1: true, 2: aperture
  spherical: f32;// 0: false, 1: true
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

/// all the Elements of the Lens under test
struct Elements {
  el: [[stride(72)]] array<Element>;
};

[[group(0), binding(0)]] var<storage, read> elements : Elements;

[[group(1), binding(2)]] var<uniform> params : SimParams;
[[group(1), binding(1)]] var<storage, read> sensor : Sensor;

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
    var rgb = lookup_rgb(in.wavelength);
    rgb.g = rgb.g * 0.6;

    var out: VertexOutput;
    // out.clip_position = vec4<f32>(pos, 0.,1.);
    out.clip_position = vec4<f32>(in.pos / 16.0 * params.zoom, 0.,1.);
    // out.clip_position = vec4<f32>(0.5, 0.5, 0.,1.);
    out.strength = in.strength;
    out.rgb = rgb;
    out.aperture_pos = in.aperture_pos;
    out.entry_pos = in.entry_pos;
    return out;
}

let tpi: f32 = 6.283185307179586;
fn clip_ray_poly(pos: vec2<f32>, num_edge: u32, size: f32) -> bool {
    var clipped = false;
    for (var i = u32(0); i < num_edge; i = i + u32(1)) {
        let part = f32(i) * tpi / f32(num_edge);
        let dir = vec2<f32>(cos(part), sin(part));

        let dist = dot(dir, pos);
        clipped = clipped || (dist > size);
    }
    return clipped;
}

fn isNan( val: f32 ) -> bool {
  if ( val < 0.0 || 0.0 < val || val == 0.0 ) {
    return false;
  } else {
    return true;
  }
}

[[stage(fragment)]]
fn mainf(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  for (var i = u32(0); i < arrayLength(&elements.el); i = i + u32(1)) {
      let element = elements.el[i];
      if (element.entry > 1.) {
          if (clip_ray_poly(in.aperture_pos, u32(element.b1), element.radius)) {
              return vec4<f32>(0., 0., 0., 0.);
          }
      }
  }

  let strength = in.strength;

  if (isNan(strength) || length(in.entry_pos) > 1.) {
    return vec4<f32>(0., 0., 0., 0.);
  }

  let s = strength * params.opacity;
  
  return vec4<f32>(in.rgb, s);
}