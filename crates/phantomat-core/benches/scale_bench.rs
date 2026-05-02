use criterion::{black_box, criterion_group, criterion_main, Criterion};
use phantomat_core::scale::{LinearScale, LogScale, Scale};

fn bench_linear_apply(c: &mut Criterion) {
    let s = LinearScale::new((0.0, 1.0), (0.0, 100.0));
    c.bench_function("linear_apply_1M", |b| {
        b.iter(|| {
            let mut acc = 0.0_f64;
            for i in 0..1_000_000 {
                let v = (i as f64) * 1e-6;
                acc += black_box(s.apply(black_box(v)));
            }
            black_box(acc)
        });
    });
}

fn bench_log_apply(c: &mut Criterion) {
    let s = LogScale::new((1.0, 1000.0), (0.0, 1.0), 10.0).unwrap();
    c.bench_function("log_apply_1M", |b| {
        b.iter(|| {
            let mut acc = 0.0_f64;
            for i in 1..=1_000_000 {
                let v = 1.0 + (i as f64) * 0.001;
                acc += black_box(s.apply(black_box(v)));
            }
            black_box(acc)
        });
    });
}

criterion_group!(benches, bench_linear_apply, bench_log_apply);
criterion_main!(benches);
