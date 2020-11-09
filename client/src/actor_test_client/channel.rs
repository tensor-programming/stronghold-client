use std::sync::mpsc;
use std::thread;

trait Channel<S, R>
where
    S: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Self: Send + Sync + std::marker::Sized + Copy + 'static,
{
    fn send(&self, _: S) -> R;
    fn connect(&self, input: S) -> Token<R> {
        let (tx, rx) = mpsc::channel();
        let thread_self = self.clone();

        thread::spawn(move || {
            tx.send(thread_self.send(input)).unwrap();
        });

        Token::<R>::new(rx)
    }
    fn get(&self, token: Token<R>) -> R {
        token.channel_rx.recv().unwrap()
    }
}

#[derive(Debug)]
struct Token<R> {
    channel_rx: mpsc::Receiver<R>,
    identity: String,
}

impl<R> Token<R> {
    fn new(channel_rx: mpsc::Receiver<R>) -> Self {
        Token::<R> {
            channel_rx,
            identity: String::from("test"),
        }
    }
}
