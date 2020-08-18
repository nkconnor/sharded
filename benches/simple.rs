use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dashmap::*;
use parking_lot::*;
use shard_lock::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

fn c_write_read(c: &mut Criterion) {
    let map: Arc<Shard<RwLock<HashMap<Uuid, ()>>>> = Arc::new(shard!(HashMap::new()));
    let readmap = map.clone();
    let mut readers = Vec::new();
    for _ in 0..16 {
        let readmap = readmap.clone();
        let handle = std::thread::spawn(move || {
            //
            loop {
                let uuid = Uuid::new_v4();
                let guard = readmap.read(&uuid);
                guard.get(&uuid);
            }
        });

        readers.push(handle);
    }

    c.bench_function("write_during_reads_shardmap", move |b| {
        let writemap = map.clone();
        b.iter(move || {
            //
            let uuid = Uuid::new_v4();
            let mut guard = writemap.write(&uuid);
            guard.insert(uuid, ());
        })
    });
}

fn c_write_read_dash(c: &mut Criterion) {
    let map = Arc::new(DashMap::new());
    let readmap = map.clone();
    let mut readers = Vec::new();
    for _ in 0..16 {
        let readmap = readmap.clone();
        let handle = std::thread::spawn(move || {
            //
            loop {
                readmap.get(&Uuid::new_v4());
            }
        });

        readers.push(handle);
    }

    c.bench_function("write_during_reads_dashmap", move |b| {
        let writemap = map.clone();
        b.iter(move || {
            //
            let uuid = Uuid::new_v4();
            writemap.insert(uuid, ());
        })
    });
}

fn c_write_read_n(c: &mut Criterion) {
    let map: Arc<RwLock<HashMap<Uuid, ()>>> = Arc::new(RwLock::new(HashMap::new()));
    let readmap = map.clone();
    let mut readers = Vec::new();
    for _ in 0..16 {
        let readmap = readmap.clone();
        let handle = std::thread::spawn(move || {
            //
            loop {
                let uuid = Uuid::new_v4();
                let guard = readmap.read();
                guard.get(&uuid);
            }
        });

        readers.push(handle);
    }

    c.bench_function("write_during_reads_rwlock", move |b| {
        let writemap = map.clone();
        b.iter(move || {
            //
            let uuid = Uuid::new_v4();
            let mut guard = writemap.write();
            guard.insert(uuid, ());
        })
    });
}

criterion_group!(benches, c_write_read, c_write_read_dash, c_write_read_n);
criterion_main!(benches);
