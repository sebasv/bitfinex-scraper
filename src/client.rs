use crate::message::Message;
use crate::subscriber::{Subscriber, SubscriberHandler};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type Alive = Arc<AtomicBool>;

// Our Handler struct.
// Here we explicity indicate that the Client needs a Sender,
// whereas a closure captures the Sender for us automatically.
pub struct Client {
    out: ws::Sender,
    alive: Alive,
    pairs: Vec<String>,
    path: PathBuf,
    subscribers: HashMap<usize, SubscriberHandler>,
}

impl ws::Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        println!("client opened");
        // Now we don't need to call unwrap since `on_open` returns a `Result<()>`.
        // If this call fails, it will only result in this connection disconnecting.
        for pair in self.pairs.iter() {
            self.out.send(
                json!({"event":"subscribe", "channel": "trades", "symbol": pair}).to_string(),
            )?;
        }
        Ok(())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        match msg {
            ws::Message::Text(s) => self.handle_message(&s),
            ws::Message::Binary(_) => { /* ignore binary data */ }
        };
        let alive = self.alive.load(Ordering::Relaxed);
        if !alive {
            self.close()
        } else {
            Ok(())
        }
    }
}

impl Client {
    pub fn new(pairs: Vec<String>, path: PathBuf, out: ws::Sender, alive: Alive) -> Client {
        Client {
            pairs,
            path,
            subscribers: HashMap::new(),
            out,
            alive,
        }
    }

    fn close(&mut self) -> ws::Result<()> {
        for key in self.subscribers.keys().cloned().collect::<Vec<usize>>() {
            if let Some(handler) = self.subscribers.remove(&key) {
                handler.close();
            }
        }
        self.out.close(ws::CloseCode::Normal)
    }

    fn handle_message(&mut self, s: &str) {
        if let Some(msg) = Message::try_new(&s) {
            match msg {
                Message::Subscribed { channel, id, .. } => {
                    let subscriber = Subscriber::spawn(
                        self.path
                            .join(format!("{}.{}.csv", channel, id))
                            .to_path_buf(),
                    );
                    self.subscribers.insert(id, subscriber);
                }
                Message::Unsubscribed { id, .. } => {
                    self.subscribers.remove(&id);
                }
                Message::Snapshot(id, vec) => {
                    for update in vec {
                        self.subscribers.entry(id).and_modify(|s| s.send(update));
                    }
                }
                Message::Update(id, _, update) => {
                    self.subscribers.entry(id).and_modify(|s| s.send(update));
                }
                Message::Info { .. } => info!("info message"),
                Message::HeartBeat(id, _) => info!("heartbeat for {}", id),
                Message::Error { msg, .. } => error!("received error: {}", msg),
            }
        }
    }
}
