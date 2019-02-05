mod client;
mod message;
mod subscriber;
use crate::client::Client;

#[macro_use]
extern crate serde_derive;
extern crate ctrlc;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate ws;

#[macro_use]
extern crate log;
extern crate simplelog;

extern crate clap;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use std::fs::{create_dir_all, File};
use std::io::BufReader;
use std::path::Path;

use simplelog::*;

const WS_ADDRESS: &str = "wss://api.bitfinex.com/ws/2";

fn set_up_ctrlc() -> Arc<AtomicBool> {
    let alive = Arc::new(AtomicBool::new(true));
    let alive_bool = alive.clone();
    ctrlc::set_handler(move || {
        alive.store(false, Ordering::Relaxed);
        println!("Ctrl-c hit. Terminating");
    })
    .expect("Error setting Ctrl-C handler");
    alive_bool
}

fn init_log(path: &Path) {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Warn, Config::default()).unwrap(),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(path.join("bitfinex-scraper.log")).unwrap(),
        ),
    ])
    .unwrap();
    info!("logging initialized.");
}

fn get_inputs<P: AsRef<Path>>(path: P) -> Vec<String> {
    let json: Vec<String> = serde_json::from_reader(BufReader::new(
        File::open(path).expect("input file does not exist"),
    ))
    .expect("input file was invalid");
    json
}

fn validate_output_path(s: String) -> Result<(), String> {
    let path = Path::new(&s).to_path_buf();
    if path.is_file() {
        return Err("Path points to a file, please supply an empty or new directory".to_owned());
    } else if path.is_dir() {
        if path
            .read_dir()
            .expect("Failed to check if output directory is empty")
            .count()
            > 0
        {
            return Err(
                "Path already exists and is nonempty, please supply an empty or new path "
                    .to_owned(),
            );
        }
    } else {
        create_dir_all(&path).map_err(|e| format!("Could not create output directory: {}", e))?;
    }
    Ok(())
}

fn main() {
    let app = clap::App::new("bitfinex scraper")
        .arg(
            clap::Arg::with_name("opath")
                .short("o")
                .long("output-path")
                .takes_value(true)
                .required(true)
                .help("The empty or new output directory")
                .validator(validate_output_path),
        )
        .arg(
            clap::Arg::with_name("ipath")
                .short("i")
                .long("input-path")
                .takes_value(true)
                .required(true)
                .help("The JSON file with an array of trading pairs to monitor"),
        );
    let matches = app.get_matches();

    let path = Path::new(matches.value_of("opath").unwrap()).to_path_buf();

    let input_vec = get_inputs(&matches.value_of("ipath").unwrap());

    // set up log
    init_log(&path);

    // set up ctrl-c breaker
    let alive = set_up_ctrlc();

    // launch WS client
    let client_thread = thread::spawn(move || {
        ws::connect(WS_ADDRESS, move |out| {
            Client::new(input_vec.clone(), path.clone(), out, alive.clone())
        })
        .unwrap();
    });

    client_thread.join().unwrap();
}
