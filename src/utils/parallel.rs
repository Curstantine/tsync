use std::{
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
};

use crate::errors::{Error, Result};

/// Runs `work` over `jobs` across up to `num_threads` worker threads, chunking the
/// job list so no more than `num_threads` threads are ever spawned.
///
/// Unlike a naive "propagate first error immediately" loop, this always drains all
/// results and joins every thread before returning, so a failing job can't leave
/// sibling workers running unsupervised in the background (e.g. still writing files
/// to the target directory) after `run()` has already reported failure to the caller.
///
/// If multiple jobs fail, the first error encountered (by receive order) is returned;
/// a worker thread panicking is reported as an `Error::descriptive` if no other error
/// was already recorded.
pub fn run_parallel<T, F>(jobs: Vec<T>, num_threads: usize, work: F) -> Result<()>
where
    T: Send + 'static + Clone,
    F: Fn(T) -> Result<()> + Send + Sync + 'static,
{
    if jobs.is_empty() {
        return Ok(());
    }

    let num_threads = num_threads.max(1);
    let chunk_size = jobs.len().div_ceil(num_threads).max(1);
    let work = Arc::new(work);

    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::<JoinHandle<()>>::new();

    for chunk in jobs.chunks(chunk_size) {
        let tx = tx.clone();
        let chunk = chunk.to_vec();
        let work = work.clone();

        let handle = thread::spawn(move || {
            for job in chunk {
                let result = work(job);
                let _ = tx.send(result);
            }
        });

        handles.push(handle);
    }

    drop(tx);

    // Drain all results first so we don't return early while sibling threads
    // are still mid-flight.
    let mut first_err: Option<Error> = None;
    for result in rx {
        if let Err(e) = result {
            first_err.get_or_insert(e);
        }
    }

    for handle in handles {
        if handle.join().is_err() {
            first_err.get_or_insert(Error::descriptive("A worker thread panicked"));
        }
    }

    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
