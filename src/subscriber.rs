use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

#[derive(Deserialize, Debug)]
pub struct Update(u64, u64, f64, f64);

pub struct Subscriber {
    path: PathBuf,
    rx: Receiver<Option<Update>>,
}

pub struct SubscriberHandler {
    tx: Sender<Option<Update>>,
    handle: JoinHandle<()>,
}

impl SubscriberHandler {
    pub fn send(&mut self, update: Update) {
        self.tx
            .send(Some(update))
            .unwrap_or_else(|e| error!("send to subscriber failed: {}", e));
    }

    pub fn close(self) {
        self.tx
            .send(None)
            .unwrap_or_else(|e| error!("sent kill message to dead subscriber: {}", e));
        self.handle
            .join()
            .unwrap_or_else(|e| error!("subscriber panicked: {:?}", e));
    }
}

impl Subscriber {
    pub fn listen(self) {
        let mut file = File::create(self.path).unwrap();
        writeln!(file, "ID,MTS,AMOUNT,PRICE")
            .unwrap_or_else(|e| error!("file write failed: {}", e));
        for switch in self.rx {
            match switch {
                Some(msg) => writeln!(file, "{},{},{},{}", msg.0, msg.1, msg.2, msg.3)
                    .unwrap_or_else(|e| error!("file write failed: {}", e)),
                None => break,
            }
        }
        info!("client hung up, time to die");
    }

    pub fn spawn(path: PathBuf) -> SubscriberHandler {
        let (tx, rx) = channel();
        let handle = thread::spawn(move || {
            let subscriber = Subscriber { rx, path };
            subscriber.listen();
        });
        SubscriberHandler { tx, handle }
    }
}
