[[block]]
struct SimParams {
  side_len: u32;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(3)]] alive: bool;
};

[[stage(vertex)]]
fn main(
    [[location(0)]] cell_num: u32,
    [[location(1)]] cell_alive: u32,
    [[location(2)]] position: vec2<f32>,
) -> VertexOutput {
    let grid_pos = vec2<u32>(cell_num%params.side_len, cell_num/params.side_len);
    let offset = vec2<f32>(
        f32(grid_pos[0])/f32(params.side_len), 
        f32(grid_pos[1])/f32(params.side_len)) * 2.f32;
    let pos = (position + offset - vec2<f32>(0.9,0.9));

    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.alive = cell_alive != 0u32;
    return out;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    if (in.alive){
        return vec4<f32>(1.0,1.0,1.0,1.0);
    } else {
        return vec4<f32>(0.0,0.0,0.0,1.0);
    }
}