struct VertexInput {
    [[location(0)]] position: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
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
};

[[group(1), binding(2)]] var<uniform> params : SimParams;

[[stage(vertex)]]
fn mainv(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.,1.);
    return out;
}

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn mainf(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let screenAspect = normalize(vec2<f32>(params.window_height_scaled, params.window_width_scaled)) * 2.;

    let pos = (vec2<f32>(in.clip_position.x / params.window_width_scaled, in.clip_position.y / params.window_height_scaled) + vec2<f32>(0.21,0.21)) / screenAspect;
    let sample = textureSample(t_diffuse, s_diffuse, pos);
    var bg = vec4<f32>(0.0,0.0,0.0,1.0);

    let color = bg * (1.0 - sample.a) + sample;
    return color;
}
