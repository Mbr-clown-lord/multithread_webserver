use crossbeam::channel;
use std::thread;
use tracing::{error, info};

type Job = Box<dyn FnOnce() + Send + 'static>;
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<channel::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = channel::unbounded();

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        if let Some(ref sender) = self.sender {
            if let Err(e) = sender.send(job) {
                let err = format!("Failed to send job {}", e);
                error!(err);
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        info!("Shutting down thread pool");
        drop(self.sender.take());

        for worker in &mut self.workers {
            let info = format!("Shutting down worker {}", worker.id);
            info!(info);
            if let Some(thread) = worker.thread.take() {
                if let Err(e) = thread.join() {
                    let err = format!("Worker {} panicked: {:?}", worker.id, e);
                    error!(err);
                }
            }
        }
    }
}
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: channel::Receiver<Job>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = match receiver.recv() {
                    Ok(job) => job,
                    Err(e) => {
                        let err = format!("Failed to recv job {}", e);
                        error!(err);
                        break;
                    }
                };

                println!("Worker {id}  got a job, executing");
                let info = format!("Worker {id} got a job, executing");
                info!(info);
                job();
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
