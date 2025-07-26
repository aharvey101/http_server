use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        println!("Worker {} got a job; executing.", id);
                        job();
                    }
                    Message::Terminate => {
                        println!("Worker {} was told to terminate.", id);
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    active_connections: Arc<AtomicUsize>,
    max_connections: usize,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    /// max_connections is the maximum number of concurrent connections allowed.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize, max_connections: usize) -> ThreadPool {
        assert!(size > 0);
        assert!(max_connections > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        let active_connections = Arc::new(AtomicUsize::new(0));

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { 
            workers, 
            sender,
            active_connections,
            max_connections,
        }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), &'static str>
    where
        F: FnOnce() + Send + 'static,
    {
        // Check if we've reached the maximum number of connections
        let current_connections = self.active_connections.load(Ordering::SeqCst);
        if current_connections >= self.max_connections {
            return Err("Maximum connections reached");
        }

        // Increment connection counter
        self.active_connections.fetch_add(1, Ordering::SeqCst);

        let active_connections = Arc::clone(&self.active_connections);
        let job = Box::new(move || {
            f();
            // Decrement connection counter when job is done
            active_connections.fetch_sub(1, Ordering::SeqCst);
        });

        self.sender.send(Message::NewJob(job)).unwrap();
        Ok(())
    }

    pub fn get_active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    pub fn get_max_connections(&self) -> usize {
        self.max_connections
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
