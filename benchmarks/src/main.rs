mod adapters;

use adapters::*;
use bustle::*;
use std::thread::sleep;
use std::time::Duration;

#[macro_use]
extern crate tracing;
extern crate tracing_serde;

fn gc_cycle() {
    sleep(Duration::from_millis(20000));
    let mut new_guard = crossbeam_epoch::pin();
    new_guard.flush();
    for _ in 0..32 {
        new_guard.repin();
    }
    let mut old_guard = crossbeam_epoch_old::pin();
    old_guard.flush();

    for _ in 0..32 {
        old_guard.repin();
    }
}

fn bench<W, T: Collection>(work: W, kind: &str)
where
    <T::Handle as CollectionHandle>::Key: Send + std::fmt::Debug,
    W: Fn(usize) -> Workload,
{
    for n in 1..num_cpus::get() {
        let span = info_span!("kind", kind = kind);
        let _guard = span.enter();

        let _res = work(n).run::<T>();
        gc_cycle();
    }
}

fn main() {
    tracing_subscriber::fmt().json().flatten_event(true).init();

    let workloads = vec![
        ("read_heavy", Mix::read_heavy()),
        ("write_heavy", Mix::insert_heavy()),
        ("update_heavy", Mix::update_heavy()),
        ("uniform", Mix::uniform()),
    ];

    for (task, mix) in workloads {
        let span = info_span!("task", task = task);
        let _guard = span.enter();

        let work = |n: usize| -> Workload { Workload::new(n, mix) };

        // random seed is used in each run
        // not sure what impact it has, but it probably doesn't hurt to
        // run this a handful of times..
        for trial_num in 0..1 {
            let span = info_span!("trial_num", trial_num = trial_num);
            let _guard = span.enter();
            bench::<_, ShardTable<u64>>(work, "Sharded");
            bench::<_, ContrieTable<u64>>(work, "Contrie");
            bench::<_, DashMapTable<u64>>(work, "DashMap");
            bench::<_, FlurryTable>(work, "Flurry");
        }
        // seems like these are slow outliers
        //bench::<_, CHashMapTable<u64>>(work);
        //bench::<_, MutexStdTable<u64>>(work);
    }
}
