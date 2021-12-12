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

[[block]]
struct SimParams {
  opacity: f32;
  width: f32;
  height: f32;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;

fn wave_length_to_rgb(wavelength: f32) -> vec3<f32> {
    // convert from µm to nm
    let wavelength = wavelength * 1000.;
    let gamma = 0.80;
    var factor: f32;
    var red: f32;
    var green: f32;
    var blue: f32;

    var done = false;

    if ((wavelength >= 380.) && (wavelength < 440.)) {
        red = -(wavelength - 440.) / (440. - 380.);
        green = 0.0;
        blue = 1.0;
        done = true;
    } if ((wavelength >= 440.) && (wavelength < 490.) && !done) {
        red = 0.0;
        green = (wavelength - 440.) / (490. - 440.);
        blue = 1.0;
        done = true;
    } if ((wavelength >= 490.) && (wavelength < 510.) && !done) {
        red = 0.0;
        green = 1.0;
        blue = -(wavelength - 510.) / (510. - 490.);
        done = true;
    } if ((wavelength >= 510.) && (wavelength < 580.) && !done) {
        red = (wavelength - 510.) / (580. - 510.);
        green = 1.0;
        blue = 0.0;
        done = true;
    } if ((wavelength >= 580.) && (wavelength < 645.) && !done) {
        red = 1.0;
        green = -(wavelength - 645.) / (645. - 580.);
        blue = 0.0;
        done = true;
    } if ((wavelength >= 645.) && (wavelength < 781.) && !done) {
        red = 1.0;
        green = 0.0;
        blue = 0.0;
        done = true;
    } if (!done) {
        red = 0.01;
        green = 0.01;
        blue = 0.01;
        done = true;
    }

    // Let the intensity fall off near the vision limits
    var done = false;
    if ((wavelength >= 380.) && (wavelength < 420.)) {
        factor = 0.3 + 0.7 * (wavelength - 380.) / (420. - 380.);
    } if ((wavelength >= 420.) && (wavelength < 701.) && !done) {
        factor = 1.0;
    } if ((wavelength >= 701.) && (wavelength < 781.) && !done) {
        factor = 0.3 + 0.7 * (780. - wavelength) / (780. - 700.);
    } if (!done) {
        factor = 0.01;
    }

    return vec3<f32>(
        pow(red * factor, gamma),
        pow(green * factor, gamma),
        pow(blue * factor, gamma)
    );
}

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    let screenAspect = normalize(vec2<f32>(params.height, params.width));

    var out: VertexOutput;
    out.clip_position = vec4<f32>(vec2<f32>(in.o.z, in.o.y) / 4.0 * screenAspect, 0.,1.);
    out.strength = in.strength;
    out.wavelength = in.wavelength;
    return out;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let s = in.strength * params.opacity;
  //wave_length_to_rgb(0.5)
  return vec4<f32>(wave_length_to_rgb(in.wavelength), sqrt(in.strength) * params.opacity);
  // return vec4<f32>(1.0, 1.0, 1.0, 0.0);
}
