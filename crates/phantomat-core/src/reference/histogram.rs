//! Single-threaded CPU histograms (oracle / tests).

use crate::color::heatmap_weight_contrib_u32;

/// Returns counts per bin; values outside `[range.0, range.1]` are ignored.
/// For degenerate `range` (`max <= min` or non-finite), returns a zeroed vector of length `bins`.
#[must_use]
pub fn histogram_1d_cpu(values: &[f64], bins: usize, range: (f64, f64)) -> Vec<u32> {
    let mut out = vec![0u32; bins.max(1)];
    if bins == 0 {
        return Vec::new();
    }
    let (min, max) = range;
    let w = max - min;
    if !w.is_finite() || w <= 0.0 {
        return out;
    }
    for &v in values {
        if let Some(i) = bin_index_1d(v, min, max, bins) {
            out[i] += 1;
        }
    }
    out
}

/// Same bin assignment as the WebGPU histogram shader: coordinates and ranges are **`f32`**.
/// Compare against GPU histogram tests; see [`histogram_2d_cpu`] for a pure `f64` oracle.
#[must_use]
pub fn histogram_2d_cpu_wgpu_semantics(
    xs: &[f64],
    ys: &[f64],
    bins_x: usize,
    bins_y: usize,
    range: ((f64, f64), (f64, f64)),
) -> Vec<Vec<u32>> {
    let mut grid: Vec<Vec<u32>> = (0..bins_y.max(1))
        .map(|_| vec![0u32; bins_x.max(1)])
        .collect();
    if bins_x == 0 || bins_y == 0 {
        return Vec::new();
    }
    let ((xmin, xmax), (ymin, ymax)) = range;
    let xr = (xmin as f32, xmax as f32);
    let yr = (ymin as f32, ymax as f32);
    let wx = xr.1 - xr.0;
    let wy = yr.1 - yr.0;
    if wx <= 0.0 || wy <= 0.0 || !wx.is_finite() || !wy.is_finite() {
        return grid;
    }
    let n = xs.len().min(ys.len());
    for i in 0..n {
        let px = xs[i] as f32;
        let py = ys[i] as f32;
        if px < xr.0 || px > xr.1 || py < yr.0 || py > yr.1 {
            continue;
        }
        let tx = (px - xr.0) / wx;
        let ty = (py - yr.0) / wy;
        let mut bx = (tx * bins_x as f32).floor() as u32;
        let mut by = (ty * bins_y as f32).floor() as u32;
        if bx >= bins_x as u32 {
            bx = bins_x as u32 - 1;
        }
        if by >= bins_y as u32 {
            by = bins_y as u32 - 1;
        }
        grid[by as usize][bx as usize] += 1;
    }
    grid
}

/// 2D histogram: outer index is **y** (row), inner is **x** (column), shape `[bins_y][bins_x]`.
#[must_use]
pub fn histogram_2d_cpu(
    xs: &[f64],
    ys: &[f64],
    bins_x: usize,
    bins_y: usize,
    range: ((f64, f64), (f64, f64)),
) -> Vec<Vec<u32>> {
    let mut grid: Vec<Vec<u32>> = (0..bins_y.max(1))
        .map(|_| vec![0u32; bins_x.max(1)])
        .collect();
    if bins_x == 0 || bins_y == 0 {
        return Vec::new();
    }
    let ((xmin, xmax), (ymin, ymax)) = range;
    let wx = xmax - xmin;
    let wy = ymax - ymin;
    if !wx.is_finite() || !wy.is_finite() || wx <= 0.0 || wy <= 0.0 {
        return grid;
    }
    let n = xs.len().min(ys.len());
    for i in 0..n {
        let x = xs[i];
        let y = ys[i];
        if let (Some(bx), Some(by)) = (
            bin_index_1d(x, xmin, xmax, bins_x),
            bin_index_1d(y, ymin, ymax, bins_y),
        ) {
            grid[by][bx] += 1;
        }
    }
    grid
}

/// 2D **weighted** aggregation with the same binning and fixed-point weights as the heatmap compute shader.
#[must_use]
pub fn heatmap_2d_weighted_wgpu_semantics(
    xs: &[f64],
    ys: &[f64],
    weights: &[f32],
    bins_x: usize,
    bins_y: usize,
    range: ((f64, f64), (f64, f64)),
) -> Vec<Vec<u32>> {
    let mut grid: Vec<Vec<u32>> = (0..bins_y.max(1))
        .map(|_| vec![0u32; bins_x.max(1)])
        .collect();
    if bins_x == 0 || bins_y == 0 {
        return Vec::new();
    }
    let ((xmin, xmax), (ymin, ymax)) = range;
    let xr = (xmin as f32, xmax as f32);
    let yr = (ymin as f32, ymax as f32);
    let wx = xr.1 - xr.0;
    let wy = yr.1 - yr.0;
    if wx <= 0.0 || wy <= 0.0 || !wx.is_finite() || !wy.is_finite() {
        return grid;
    }
    let n = xs.len().min(ys.len()).min(weights.len());
    for i in 0..n {
        let px = xs[i] as f32;
        let py = ys[i] as f32;
        let c = heatmap_weight_contrib_u32(weights[i]);
        if c == 0 {
            continue;
        }
        if px < xr.0 || px > xr.1 || py < yr.0 || py > yr.1 {
            continue;
        }
        let tx = (px - xr.0) / wx;
        let ty = (py - yr.0) / wy;
        let mut bx = (tx * bins_x as f32).floor() as u32;
        let mut by = (ty * bins_y as f32).floor() as u32;
        if bx >= bins_x as u32 {
            bx = bins_x as u32 - 1;
        }
        if by >= bins_y as u32 {
            by = bins_y as u32 - 1;
        }
        grid[by as usize][bx as usize] = grid[by as usize][bx as usize].saturating_add(c);
    }
    grid
}

fn bin_index_1d(v: f64, min: f64, max: f64, bins: usize) -> Option<usize> {
    if !v.is_finite() || !min.is_finite() || !max.is_finite() || bins == 0 {
        return None;
    }
    if v < min || v > max {
        return None;
    }
    let w = max - min;
    let t = (v - min) / w;
    if !t.is_finite() {
        return None;
    }
    // Map [min, max] into bins; v == max lands in last bin.
    let mut b = (t * bins as f64).floor() as usize;
    if b >= bins {
        b = bins - 1;
    }
    Some(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn one_d_manual_uniform_bins() {
        let v = [0.0, 0.25, 0.5, 0.75, 1.0];
        let h = histogram_1d_cpu(&v, 4, (0.0, 1.0));
        assert_eq!(h, vec![1, 1, 1, 2]);
    }

    #[test]
    fn one_d_out_of_range_ignored() {
        let v = [-1.0, 0.5, 2.0];
        let h = histogram_1d_cpu(&v, 2, (0.0, 1.0));
        assert_eq!(h, vec![0, 1]);
    }

    #[test]
    fn two_d_single_bin_corner() {
        let xs = [0.0];
        let ys = [0.0];
        let g = histogram_2d_cpu(&xs, &ys, 1, 1, ((-1.0, 1.0), (-1.0, 1.0)));
        assert_eq!(g, vec![vec![1u32]]);
    }

    #[test]
    fn two_d_quadrants() {
        let xs = [-0.5, 0.5, -0.5, 0.5];
        let ys = [-0.5, -0.5, 0.5, 0.5];
        let g = histogram_2d_cpu(&xs, &ys, 2, 2, ((-1.0, 1.0), (-1.0, 1.0)));
        assert_eq!(
            g,
            vec![
                vec![1, 1],
                vec![1, 1], // row0 y negative in our mapping: ymin=-1 ymax=1 -> row0 is low y
            ]
        );
    }

    #[test]
    fn degenerate_range_returns_zeros_but_sized() {
        let h = histogram_1d_cpu(&[1.0, 2.0], 3, (0.0, 0.0));
        assert_eq!(h, vec![0, 0, 0]);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn one_d_sum_equals_in_range(points in prop::collection::vec(-2.0f64..2.0f64, 0..500),
                                     bins in 1usize..32,
                                     lo in -1.0f64..1.0,
                                     hi in 1.0f64..3.0) {
            prop_assume!(hi > lo);
            let range = (lo, hi);
            let h = histogram_1d_cpu(&points, bins, range);
            let sum: u32 = h.iter().sum();
            let in_range = points.iter().filter(|&&v| v >= lo && v <= hi && v.is_finite()).count() as u32;
            prop_assert_eq!(sum, in_range);
        }
    }
}
