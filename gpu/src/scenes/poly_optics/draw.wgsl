struct VertexInput {
    [[location(0)]] o: vec3<f32>;
    [[location(1)]] d: vec3<f32>;
    [[location(2)]] strength: f32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] strength: f32;
};

[[block]]
struct SimParams {
  opacity: f32;
  width: f32;
  height: f32;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    let screenAspect = normalize(vec2<f32>(params.height, params.width));

    var out: VertexOutput;
    out.clip_position = vec4<f32>(vec2<f32>(in.o.z, in.o.y) / 4.0 * screenAspect, 0.,1.);
    out.strength = in.strength;
    return out;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  let s = in.strength * params.opacity;
    return vec4<f32>(1.0, 1.0, 1.0, sqrt(in.strength) * params.opacity);
    // return vec4<f32>(1.0, 1.0, 1.0, 0.0);
}
