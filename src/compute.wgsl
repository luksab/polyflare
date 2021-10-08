struct Cell {
  number: u32;
  alive : u32;
};

[[block]]
struct SimParams {
  side_len: u32;
};

[[block]]
struct Cells {
  cells : [[stride(8)]] array<Cell>;
};

[[group(0), binding(0)]] var<uniform> params : SimParams;
[[group(0), binding(1)]] var<storage, read> cellsSrc : Cells;
[[group(0), binding(2)]] var<storage, read_write> cellsDst : Cells;

fn grid_to_index(grid_pos: vec2<u32>) -> u32 {
  return grid_pos[0] + grid_pos[1] * params.side_len;
}

fn index_to_grid(index: u32) -> vec2<u32> {
  let grid_pos = vec2<u32>(index%params.side_len, index/params.side_len);
  return grid_pos;
}

fn count_neighbours(pos: vec2<i32>) -> u32 {
  var count = 0u32;
  for (var i: i32 = -1; i <= 1; i = i + 1) {
    for (var j: i32 = -1; j <= 1; j = j + 1) {
      if (i != 0 || j != 0) {
        if (cellsSrc.cells[grid_to_index(vec2<u32>(pos - vec2<i32>(i,j)))].alive > 0u32) {
          count = count + 1u32;
        }
      }
    }
  }
  return count;
}

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
  let total = arrayLength(&cellsSrc.cells);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  var old : u32 = cellsSrc.cells[index].alive;

  var alive = 0u32;

  let neighbours = count_neighbours(vec2<i32>(index_to_grid(index)));
  // if (neighbours < 2u32) {
  //   alive = 0u32;
  // } else { if (neighbours == 3u32 && old == 0u32) {
  //   alive = 100u32;
  // } else { if (neighbours == 2u32 || neighbours == 3u32) {
  //   alive = old;
  // } else { if (neighbours > 3u32) {
  //   alive = 0u32;
  // }}}}

  if ((old != 0u32 && neighbours == 2u32 || neighbours == 3u32) || 
      (old == 0u32 && neighbours == 3u32)) {
    alive = 1u32;
  } else {
    alive = 0u32;
  }

  // Write back
  cellsDst.cells[index].alive = alive;
}
