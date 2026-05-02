@group(0) @binding(0) var<storage, read_write> bins_atomic: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> bins_plain: array<u32>;

struct Params {
    n_bins: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n_bins) {
        return;
    }
    let v = atomicLoad(&bins_atomic[i]);
    bins_plain[i] = v;
}
