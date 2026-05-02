//! Instanced scatter disks in NDC.
//!
//! Blend (configured on the pipeline, not here): **standard alpha-over** —
//! non-premultiplied RGB with `SrcAlpha` / `OneMinusSrcAlpha`. Fragment outputs
//! `vec4(rgb, a * coverage)` so edges composite correctly over the cleared background.

struct CanvasUniform {
    size_px: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u_canvas: CanvasUniform;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) quad: vec2<f32>,
    @location(1) inst_pos: vec2<f32>,
    @location(2) inst_color: vec4<f32>,
    @location(3) inst_size_px: f32,
) -> VertexOutput {
    let rx = inst_size_px / u_canvas.size_px.x;
    let ry = inst_size_px / u_canvas.size_px.y;
    let ndc = inst_pos + vec2<f32>(quad.x * rx, quad.y * ry);
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.local_pos = quad * 0.5;
    out.color = inst_color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.local_pos);
    let aa = fwidth(dist) * 0.5;
    let alpha = 1.0 - smoothstep(0.5 - aa, 0.5 + aa, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
