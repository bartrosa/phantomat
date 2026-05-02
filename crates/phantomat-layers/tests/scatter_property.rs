use phantomat_layers::ScatterLayer;
use phantomat_renderer::HeadlessRenderer;
use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        .. ProptestConfig::default()
    })]

    #[test]
    fn scatter_png_nonempty(
        n in 1usize..80usize,
        seed in any::<u64>(),
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut positions = Vec::with_capacity(n);
        let mut colors = Vec::with_capacity(n);
        let mut sizes = Vec::with_capacity(n);
        for _ in 0..n {
            positions.push([rng.gen_range(-0.95..0.95), rng.gen_range(-0.95..0.95)]);
            colors.push([rng.gen_range(0.05..1.0), rng.gen_range(0.05..1.0), rng.gen_range(0.05..1.0), 1.0]);
            sizes.push(rng.gen_range(2.0_f32..32.0));
        }
        let layer = ScatterLayer::new(positions, colors, sizes, (256, 256));
        let r = HeadlessRenderer::new(256, 256).expect("headless");
        let png = r.render_to_png(&layer).expect("render");
        prop_assert!(png.len() >= 100);
    }
}
