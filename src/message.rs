use crate::subscriber::Update as SingleUpdate;
use serde_json::from_str;

#[derive(Deserialize, Debug)]
pub struct Platform {
    status: i32,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Message {
    Info {
        event: String,
        version: i32,
        #[serde(rename = "serverId")]
        server_id: String,
        platform: Platform,
    },
    Subscribed {
        event: String,
        channel: String,
        #[serde(rename = "chanId")]
        id: usize,
        symbol: String,
        pair: String,
    },
    Unsubscribed {
        event: String,
        status: String,
        #[serde(rename = "chanId")]
        id: usize,
    },
    Error {
        event: String,
        msg: String,
        code: i32,
    },
    Pong {
        event: String, //"pong",
        ts: u64,       //1511545528111,
        cid: u32,      //1234
    },
    // [
    //     CHANNEL_ID,
    //     [
    //         [ UPDATE_MESSAGE ],
    //         ...
    //     ]
    // ]
    Snapshot(usize, Vec<SingleUpdate>),

    // [
    //     CHANNEL_ID,
    //     <"tu", "te">,
    //     [ UPDATE_MESSAGE ],
    // ]
    // contrary to what the documentation says, the message includes a string {"tu", "te"}
    Update(usize, String, SingleUpdate),

    // [ CHANNEL_ID, "hb" ]
    HeartBeat(usize, String),
}

impl Message {
    pub fn try_new(s: &str) -> Option<Message> {
        if let Ok(msg) = from_str::<Message>(s) {
            Some(msg)
        } else {
            error!("Could not decode json:\t{}", s);
            None
        }
    }
}
