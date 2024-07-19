use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    thread::{JoinHandle, ThreadId},
};

pub type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct TaskPoolState {
    size: usize,
    running: HashMap<ThreadId, JoinHandle<()>>,
    queue: VecDeque<Task>,
}

impl TaskPoolState {
    pub fn new(size: usize) -> Self {
        TaskPoolState {
            size,
            running: HashMap::new(),
            queue: VecDeque::new(),
        }
    }
}

pub struct TaskPool {
    state: Arc<Mutex<TaskPoolState>>,
}

impl TaskPool {
    pub fn new(size: usize) -> Self {
        TaskPool {
            state: Arc::new(Mutex::new(TaskPoolState::new(size))),
        }
    }

    pub fn spawn(&self, task: impl FnOnce() + Send + 'static) {
        let mut state = self.state.lock().unwrap();
        state.queue.push_back(Box::new(task));
        drop(state);
        TaskPool::run_one(Arc::clone(&self.state));
    }

    fn run_one(state: Arc<Mutex<TaskPoolState>>) {
        let mut locked = state.lock().unwrap();
        if locked.running.len() >= locked.size {
            return;
        }

        if let Some(task) = locked.queue.pop_front() {
            let inner = Arc::clone(&state);
            let handle = std::thread::spawn(move || {
                task();
                let mut state = inner.lock().unwrap();
                state.running.remove(&std::thread::current().id());
                drop(state);
                TaskPool::run_one(inner);
            });

            locked.running.insert(handle.thread().id(), handle);
        }
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        let mut running = match self.state.lock() {
            Ok(mut state) => std::mem::take(&mut state.running),
            Err(_) => return,
        };
        while !running.is_empty() {
            for (_, handle) in running {
                handle.join().unwrap();
            }

            let mut state = self.state.lock().unwrap();
            running = std::mem::take(&mut state.running);
        }
    }
}

pub type ScopedTask<'a> = Box<dyn FnOnce() + Send + 'a>;

pub struct ScopedTaskPool<'a> {
    size: usize,
    queue: VecDeque<ScopedTask<'a>>,
}

impl<'a> ScopedTaskPool<'a> {
    pub fn new(size: usize) -> Self {
        ScopedTaskPool {
            size,
            queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: impl FnOnce() + Send + 'a) -> &mut Self {
        self.queue.push_back(Box::new(task));
        self
    }

    pub fn run(&mut self) {
        while !self.queue.is_empty() {
            let len = self.queue.len().min(self.size);
            let tasks = self.queue.drain(..len).collect::<Vec<_>>();
            std::thread::scope(move |scope| {
                for task in tasks {
                    scope.spawn(|| task());
                }
            });
        }
    }
}
