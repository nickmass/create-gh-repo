#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate serde;
extern crate serde_json;
extern crate tempfile;
extern crate hyper;
extern crate git2;
extern crate url;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate notify;

mod cli;
mod error;
use error::{Error, Result};
mod http;
use http::HttpClient;
mod git;
use git::GitMode;

use std::thread;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::{Arc, Mutex};
use std::process::Command;
use std::io::{Write, Read, Seek, SeekFrom};

use tempfile::NamedTempFile;
use serde_json as json;
use notify::{Watcher, RecommendedWatcher};

#[derive(Serialize, Deserialize, Debug)]
struct CreateRequest {
    name: String,
    description: String,
    homepage: String,
    private: bool,
    has_issues: bool,
    has_wiki: bool,
    has_downloads: bool,
    auto_init: bool,
    gitignore_template: String,
    license_template: String,
}

impl Default for CreateRequest {
    fn default() -> CreateRequest {
       CreateRequest {
            name: "".into(),
            description: "".into(),
            homepage: "".into(),
            private: false,
            has_issues: true,
            has_wiki: false,
            has_downloads: false,
            auto_init: true,
            gitignore_template: "".into(),
            license_template: "".into(),
       }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateResponse {
    clone_url: String,
}

fn main() {
    env_logger::init().map_err(error).unwrap();

    let options = cli::get_options().map_err(error).unwrap();
    let request_params = prompt_create_params(&options.editor).map_err(error).unwrap();
    
    if request_params.is_none() {
        println!("Request parameters not saved, repository not created.");
        return;
    }

    let request_params = request_params.unwrap();

    let api_url = "https://api.github.com/user/repos";
    let mut client = HttpClient::new();
    client.with_basic_authorization(options.auth, "");
    let res: CreateResponse = client.post_object(api_url, &request_params).map_err(error).unwrap();
    
    let dir = options.directory.as_ref().map(|x| &**x);
    println!("Repository Created: {}", res.clone_url);
    match options.mode {
        GitMode::Create => {},
        GitMode::Clone => {
            let clone_dir = git::clone(&res.clone_url, dir).map_err(error).unwrap();
            println!("Cloned into: {}", clone_dir);
        },
        GitMode::Remotes => {unimplemented!();},
        GitMode::Push => {unimplemented!();},
        GitMode::Rebase => {unimplemented!();},
    }
}

#[allow(unreachable_code)]
fn error<E>(err: E) -> E 
    where E: std::error::Error {
    error!("Error: {:?}", err);
    println!("Error: {}", err.description());
    std::process::exit(1);
    err
}

fn prompt_create_params(editor: &str) -> Result<Option<CreateRequest>> {
    let mut tmp_file = try!(NamedTempFile::new());
    let _ = write!(tmp_file, "{}", template_text());
    let _ = tmp_file.sync_all();
    let path = try!(tmp_file.path().to_str().ok_or(Error::InvalidTargetDir));

    let (tx, rx) = channel();
    let closed = Arc::new(Mutex::new(false));
    let written = Arc::new(Mutex::new(false));

    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx));
    {
        let closed = closed.clone();
        let written = written.clone();
        let path = path.to_string();
        thread::spawn(move || -> Result<()> {
            try!(watcher.watch(path));
            loop {
                {
                    let closed = closed.lock().unwrap();
                    if *closed { return Ok(()) }
                }
                match rx.try_recv() {
                    Err(TryRecvError::Disconnected) => { return Ok(()) },
                    Err(TryRecvError::Empty) => {},
                    Ok(notify::Event{op: Ok(notify::op::WRITE), ..}) => {
                        let mut written = written.lock().unwrap();
                        *written = true;
                    },
                    _ => {},
                }
            }
        });
    }
    
    let status = try!(Command::new(editor).arg(path).status());
    {
        let mut closed = closed.lock().unwrap();
        *closed = true;
    }

    let written = written.lock().unwrap();
    if !status.success() || !*written { return Ok(None); }
    let mut tmp_file = try!(tmp_file.reopen());
    let _ = tmp_file.seek(SeekFrom::Start(0));
    let mut text = String::new();
    let _ = tmp_file.read_to_string(&mut text);
    Ok(Some(try!(json::from_str(&*text))))
}

fn template_text() -> String {
    let json = json::to_string_pretty(&CreateRequest::default()).unwrap();
    json
}
