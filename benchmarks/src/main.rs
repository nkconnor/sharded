#![feature(core_intrinsics)]
mod adapters;

use adapters::{CHashMapTable, ContrieTable, DashMapTable, FlurryTable, MutexStdTable, ShardTable};
use bustle::*;
use std::thread::sleep;
use std::time::Duration;

#[macro_use]
extern crate tracing;
extern crate tracing_serde;

// no explanation of this in original repo that I can find. looks like
// different allocators have been in and out of there
// #[global_allocator]
// static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

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

fn bench<W, T: Collection>(work: W)
where
    <T::Handle as CollectionHandle>::Key: Send + std::fmt::Debug,
    W: Fn(usize) -> Workload,
{
    let kind = unsafe { std::intrinsics::type_name::<T>() };
    for n in 1..num_cpus::get() {
        let span = info_span!("kind", kind = kind);
        let _guard = span.enter();

        let _res = work(n).run::<T>();
        gc_cycle();
    }
}

//fn rapid_grow_task() {
//    for n in 1..=num_cpus::get() {
//        rapid_grow(n).run::<MutexStdTable<u64>>();
//        gc_cycle();
//    }
//    for n in 1..=num_cpus::get() {
//        rapid_grow(n).run::<CHashMapTable<u64>>();
//        gc_cycle();
//    }
//    for n in 1..=num_cpus::get() {
//        rapid_grow(n).run::<FlurryTable>();
//        gc_cycle();
//    }
//    for n in 1..=num_cpus::get() {
//        rapid_grow(n).run::<ContrieTable<u64>>();
//        gc_cycle();
//    }
//    for n in 1..=num_cpus::get() {
//        rapid_grow(n).run::<DashMapTable<u64>>();
//        gc_cycle();
//    }
//}

fn main() {
    tracing_subscriber::fmt().json().flatten_event(true).init();

    let workloads = vec![
        //        ("read_heavy", Mix::read_heavy()),
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
            bench::<_, ContrieTable<u64>>(work);
            bench::<_, DashMapTable<u64>>(work);
            bench::<_, ShardTable<u64>>(work);
            bench::<_, MutexStdTable<u64>>(work);
            bench::<_, FlurryTable>(work);
        }
        // seems like this is an outlier
        //bench::<_, CHashMapTable<u64>>(work);
    }
}
