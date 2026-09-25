//! Measures the full `unit:upsert` call path — string marshalling,
//! `FromArma` parsing, and the `StateCache` write — since that's what runs
//! once per tracked unit, per tick, in `fnc_pushState`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use minerva::init;

fn quoted(s: &str) -> String {
    format!("\"{s}\"")
}

fn bench_unit_upsert(c: &mut Criterion) {
    let extension = init().testing();
    let (_, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);

    c.bench_function("unit_upsert", |b| {
        b.iter(|| {
            let args = vec![
                quoted("u1"),
                quoted("g1"),
                quoted("INFANTRY"),
                quoted("B_Soldier_F"),
                "[100,200,300]".to_string(),
                "45".to_string(),
                "[1,2,3]".to_string(),
                "0.1".to_string(),
            ];
            black_box(extension.call("unit:upsert", Some(args)));
        });
    });
}

criterion_group!(benches, bench_unit_upsert);
criterion_main!(benches);
