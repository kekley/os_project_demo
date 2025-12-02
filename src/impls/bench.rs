use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

use tokio::runtime::Runtime;

fn do_work(counter: &Arc<AtomicU64>) {
    counter.fetch_add(1, Ordering::Relaxed);
    let mut s: u64 = 0;
    for _ in 0..100 {
        s = s.wrapping_add(1);
    }
    let _ = s;
}

fn bench_one_to_one(n_workers: usize, iterations: usize) -> std::time::Duration {
    use std::sync::mpsc::{SyncSender, sync_channel};
    use std::thread;

    let counter = Arc::new(AtomicU64::new(0));

    let (on_done_tx, on_done_rx) = sync_channel::<()>(0);

    let mut handles = Vec::with_capacity(n_workers);
    let mut senders: Vec<SyncSender<()>> = Vec::with_capacity(n_workers);

    for _ in 0..n_workers {
        let (tx, rx) = sync_channel::<()>(0);
        let on_done_tx = on_done_tx.clone();
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            while let Ok(()) = rx.recv() {
                do_work(&counter);
                let _ = on_done_tx.send(());
            }
        });
        senders.push(tx);
        handles.push(handle);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        for s in senders.iter() {
            let _ = s.send(());
        }
        for _ in 0..n_workers {
            let _ = on_done_rx.recv();
        }
    }
    let dur = start.elapsed();

    drop(senders);
    for h in handles {
        let _ = h.join();
    }

    dur
}

fn bench_many_to_many(n_workers: usize, iterations: usize) -> std::time::Duration {
    use tokio::sync::mpsc::{Sender, channel};

    let counter = Arc::new(AtomicU64::new(0));

    let rt = Runtime::new().expect("tokio runtime");

    rt.block_on(async {
        let (on_done_tx, mut on_done_rx) = channel::<()>(100_000);
        let mut senders: Vec<Sender<()>> = Vec::with_capacity(n_workers);

        for _ in 0..n_workers {
            let (tx, mut rx) = channel::<()>(1);
            let on_done_tx = on_done_tx.clone();
            let counter = counter.clone();
            tokio::spawn(async move {
                while let Some(()) = rx.recv().await {
                    do_work(&counter);
                    let _ = on_done_tx.send(()).await;
                }
            });
            senders.push(tx);
        }

        let start = Instant::now();
        for _ in 0..iterations {
            for s in senders.iter_mut() {
                let _ = s.send(()).await;
            }
            for _ in 0..n_workers {
                let _ = on_done_rx.recv().await;
            }
        }
        let dur = start.elapsed();

        drop(senders);
        dur
    })
}

fn bench_many_to_one(n_workers: usize, iterations: usize) -> std::time::Duration {
    let counter = Arc::new(AtomicU64::new(0));

    let mut tasks: Vec<Arc<AtomicU64>> = Vec::with_capacity(n_workers);
    for _ in 0..n_workers {
        tasks.push(counter.clone());
    }

    let start = Instant::now();
    for _ in 0..iterations {
        for t in tasks.iter() {
            do_work(t);
        }
    }
    start.elapsed()
}

pub fn run_benchmarks(n_workers: usize, iterations: usize) -> String {
    let one_to_one = bench_one_to_one(n_workers, iterations);
    let many_to_many = bench_many_to_many(n_workers, iterations);
    let many_to_one = bench_many_to_one(n_workers, iterations);

    let per_op_oto = one_to_one.as_secs_f64() / (n_workers as f64 * iterations as f64);
    let per_op_mtm = many_to_many.as_secs_f64() / (n_workers as f64 * iterations as f64);
    let per_op_mto = many_to_one.as_secs_f64() / (n_workers as f64 * iterations as f64);

    format!(
        "Benchmark results (workers = {n}, iters = {it})\n\nOne-to-One (OS threads): {oto:?} total, {per_oto:.9}s/op\nMany-to-Many (async tasks): {mtm:?} total, {per_mtm:.9}s/op\nMany-to-One (sequential): {mto:?} total, {per_mto:.9}s/op\n",
        n = n_workers,
        it = iterations,
        oto = one_to_one,
        per_oto = per_op_oto,
        mtm = many_to_many,
        per_mtm = per_op_mtm,
        mto = many_to_one,
        per_mto = per_op_mto,
    )
}
