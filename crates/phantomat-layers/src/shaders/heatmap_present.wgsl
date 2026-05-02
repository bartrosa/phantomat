struct PresentParams {
    bins: vec2<u32>,
    max_count: f32,
    _pad: u32,
    color_low: vec4<f32>,
    color_high: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> bins: array<u32>;
@group(0) @binding(1) var<uniform> params: PresentParams;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let p = pos[vid];
    var o: VsOut;
    o.clip = vec4<f32>(p, 0.0, 1.0);
    o.uv = p * 0.5 + vec2<f32>(0.5, 0.5);
    return o;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = params.bins;
    if (dims.x == 0u || dims.y == 0u) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let fx = f32(dims.x);
    let fy = f32(dims.y);
    let bx = u32(min(uv.x * fx, fx - 1.0));
    let by = u32(min(uv.y * fy, fy - 1.0));
    let idx = by * dims.x + bx;
    let v = bins[idx];
    let mf = params.max_count;
    let t = select(0.0, f32(v) / mf, mf > 0.0);
    let tc = clamp(t, 0.0, 1.0);
    return mix(params.color_low, params.color_high, tc);
}
