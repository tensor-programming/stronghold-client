use crossbeam_channel::{unbounded, Receiver, Sender};

use std::{sync::Arc, thread};

pub trait Listener<T>
where
    T: Send + Sync,
{
    fn dispatch(&self, _event: &T) {}
}

pub struct Dispatcher<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
    listeners: Vec<Arc<dyn Listener<T> + Send + Sync>>,
}

pub struct DispatchBuilder<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
    listeners: Vec<Arc<dyn Listener<T> + Send + Sync>>,
}

impl<T> DispatchBuilder<T>
where
    T: Sync + Send + Sized,
{
    pub fn new() -> Self {
        let (tx, rx) = unbounded::<T>();

        Self {
            tx,
            rx,
            listeners: vec![],
        }
    }

    pub fn add_listener(mut self, listener: Arc<dyn Listener<T> + Send + Sync>) -> Self {
        self.listeners.push(listener);

        self
    }

    pub fn build(self) -> Dispatcher<T> {
        Dispatcher {
            tx: self.tx,
            rx: self.rx,
            listeners: self.listeners,
        }
    }
}
