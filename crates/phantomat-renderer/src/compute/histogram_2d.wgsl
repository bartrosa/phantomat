struct HistParams {
    range_x: vec2<f32>,
    range_y: vec2<f32>,
    bins_x: u32,
    bins_y: u32,
    n_inputs: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> input_xy: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> bins: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: HistParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n_inputs) {
        return;
    }
    let p = input_xy[i];
    let xr = params.range_x;
    let yr = params.range_y;
    if (p.x < xr.x || p.x > xr.y || p.y < yr.x || p.y > yr.y) {
        return;
    }
    let wx = xr.y - xr.x;
    let wy = yr.y - yr.x;
    if (wx <= 0.0 || wy <= 0.0) {
        return;
    }
    let tx = (p.x - xr.x) / wx;
    let ty = (p.y - yr.x) / wy;
    var bx = u32(floor(tx * f32(params.bins_x)));
    var by = u32(floor(ty * f32(params.bins_y)));
    if (bx >= params.bins_x) {
        bx = params.bins_x - 1u;
    }
    if (by >= params.bins_y) {
        by = params.bins_y - 1u;
    }
    let idx = by * params.bins_x + bx;
    atomicAdd(&bins[idx], 1u);
}
