//! Measures the full `group:upsert` call path -- string marshalling,
//! `FromArma` parsing, and the `StateCache` write -- since that's what runs
//! once per group, per tick, in `fnc_pushGroup.sqf` (one call for the whole
//! group, its full unit list included, not one per tracked unit).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use minerva::init;

fn quoted(s: &str) -> String {
    format!("\"{s}\"")
}

/// A 12-man squad plus its vehicle: representative of the largest single
/// `group:upsert` call a real mission produces (see docs/in-game-testing.md's
/// large-group check).
const UNITS_PER_GROUP: usize = 13;

fn unit_literal(i: usize) -> String {
    format!(
        "[{},{},{},[{},{},{}],45,[1,2,3],0.1]",
        quoted(&format!("u{i}")),
        quoted("INFANTRY"),
        quoted("B_Soldier_F"),
        100.0 + i as f64,
        200.0,
        300.0,
    )
}

fn units_arg() -> String {
    format!(
        "[{}]",
        (0..UNITS_PER_GROUP)
            .map(unit_literal)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn bench_group_upsert(c: &mut Criterion) {
    let extension = init().testing();
    let (_, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);

    let units = units_arg();
    c.bench_function("group_upsert", |b| {
        b.iter(|| {
            let args = vec![
                quoted("g1"),
                quoted("WEST"),
                "[1,1,1]".to_string(),
                "false".to_string(),
                "[]".to_string(),
                units.clone(),
            ];
            black_box(extension.call("group:upsert", Some(args)));
        });
    });
}

criterion_group!(benches, bench_group_upsert);
criterion_main!(benches);
