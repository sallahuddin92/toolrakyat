use starpdf::execution::{ContentHash, ObjectSnapshot};
use starpdf::syntax::object::ObjectRef;
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    const COUNT: u64 = 10_000;
    let before = ObjectSnapshot(
        (1..=COUNT)
            .map(|n| (ObjectRef::new(n, 0), ContentHash([1; 32])))
            .collect::<BTreeMap<_, _>>(),
    );
    let mut after = before.clone();
    after
        .0
        .insert(ObjectRef::new(COUNT, 0), ContentHash([2; 32]));
    let started = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(before.diff(&after));
    }
    println!(
        "shadow delta: {} objects x 100 iterations in {:?}",
        COUNT,
        started.elapsed()
    );
}
