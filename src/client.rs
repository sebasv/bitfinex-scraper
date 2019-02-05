use crate::message::Message;
use crate::subscriber::{Subscriber, SubscriberHandler};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type Alive = Arc<AtomicBool>;

pub struct Client {
    out: ws::Sender,
    alive: Alive,
    pairs: Vec<String>,
    path: PathBuf,
    subscribers: HashMap<usize, SubscriberHandler>,
    last_ping: Instant,
}

impl ws::Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        println!("client opened");
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

        if Instant::now() - self.last_ping > Duration::from_secs(300) {
            // ping every 5 minutes to try and keep the session fresh
            self.out
                .send(json!({"event":"ping", "cid":1234}).to_string())?;
            self.last_ping = Instant::now();
        }

        self.check_close()
    }

    fn on_timeout(&mut self, _event: ws::util::Token) -> ws::Result<()> {
        warn!("timeout on websocket connection");
        self.check_close()
    }
}

impl Client {
    pub fn new(pairs: Vec<String>, path: PathBuf, out: ws::Sender, alive: Alive) -> Client {
        let client = Client {
            pairs,
            path,
            subscribers: HashMap::new(),
            out,
            alive,
            last_ping: Instant::now(),
        };
        client.out.timeout(10_000, ws::util::Token(1234)).unwrap();
        client
    }

    fn check_close(&mut self) -> ws::Result<()> {
        let alive = self.alive.load(Ordering::Relaxed);
        if !alive {
            self.close()
        } else {
            Ok(())
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
                Message::Subscribed {
                    channel, pair, id, ..
                } => {
                    let subscriber = Subscriber::spawn(
                        self.path
                            .join(format!("{}.{}.{}.csv", channel, pair, id))
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
                Message::Info { .. } => info!("info message: {:?}", msg),
                Message::HeartBeat(id, _) => info!("heartbeat for {}", id),
                Message::Error { msg, .. } => error!("received error: {}", msg),
                Message::Pong { .. } => info!("server ponged"),
            }
        }
    }
}
