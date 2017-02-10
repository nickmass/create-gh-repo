#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate tempfile;
extern crate reqwest;
extern crate git2;
extern crate url;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate notify;

mod cli;
mod git;
mod error;

use git::GitMode;
use error::{Error, Result};

use reqwest::Client;
use reqwest::header::{UserAgent, Authorization, Basic};
use serde::Serialize;
use serde_json as json;
use tempfile::NamedTempFile;
use notify::{Watcher, RecommendedWatcher};

use std::thread;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Command;
use std::io::{Write, Read};
use std::fs::{remove_file, File};

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

trait JsonTemplate
    where Self: Sized
{
    fn to_template(&self) -> String;
    fn from_template(&str) -> Result<Self>;
}

impl JsonTemplate for CreateRequest {
    fn to_template(&self) -> String {
        let mut buf = String::new();

        fn wrap<T>(item: &T) -> String
            where T: Serialize
        {
            json::to_string_pretty(item).unwrap()
        }

        buf.push_str(&*format!(r#"{{
    //Required. The name of the repository
    "name": {name},
    //A short description of the repository
    "description": {description},
    //A URL with more information about the repository
    "homepage": {homepage},
    //Set to true to create a private repository
    "private": {private},
    //Set to true to enable issues for the repository
    "has_issues": {has_issues},
    //Set to true to enable the wiki for the repository
    "has_wiki": {has_wiki},
    //Set to true to enable downloads for the repository
    "has_downloads": {has_downloads},
    //Pass true to create an initial commit with empty README
    "auto_init": {auto_init},
    //Desired language or platform .gitignore template to apply. For example, "Haskell"
    "gitignore_template": {gitignore_template},
    //Desired LICENSE template to apply. For example, "mit" or "mozilla"
    "license_template": {license_template}
}}"#,
                               name = wrap(&self.name),
                               description = wrap(&self.description),
                               homepage = wrap(&self.homepage),
                               private = wrap(&self.private),
                               has_issues = wrap(&self.has_issues),
                               has_wiki = wrap(&self.has_wiki),
                               has_downloads = wrap(&self.has_downloads),
                               auto_init = wrap(&self.auto_init),
                               gitignore_template = wrap(&self.gitignore_template),
                               license_template = wrap(&self.license_template)));

        buf
    }

    fn from_template(str: &str) -> Result<Self> {
        use nom::rest_s;
        named!(strip_comments<&str, String>, fold_many0!(alt!(chain!(
                val: take_until_s!(" //") ~
                is_not_s!("\r\n")~
                tag_s!("\r")?~
                tag_s!("\n"),
                || val) |
                rest_s),
            String::new(), |mut acc: String, item: &str| {
                acc.push_str(item);
                acc
        }));
        let result = strip_comments(str);

        if !result.is_done() {
            return Err(Error::Nom);
        }


        json::from_str(&*result.unwrap().1).map_err(|e| e.into())
    }
}


fn main() {
    env_logger::init().map_err(error).unwrap();

    let options = cli::get_options(None).map_err(error).unwrap();
    let dir = options.directory.as_ref().map(|x| &**x);
    let user = options.username.as_ref().map(|x| &**x);
    let pass = options.password.as_ref().map(|x| &**x);

    let default_params = CreateRequest {
        name: git::get_repo_name(dir).unwrap_or("".into()),
        auto_init: options.mode != GitMode::Push,
        ..Default::default()
    };
    let request_params =
        prompt_create_params(&options.editor, &default_params).map_err(error).unwrap();

    if request_params.is_none() {
        println!("Request parameters not saved, repository not created.");
        return;
    }

    let request_params = request_params.unwrap();

    // TODO: Need to handle errors from Github api
    let api_url = "https://api.github.com/user/repos";
    let client = Client::new().unwrap();
    let res: CreateResponse = client.post(api_url)
        .header(Authorization(Basic{
            username: options.auth,
            password: None
        }))
        .header(UserAgent(format!("create-gh-repo/{}", crate_version!())))
        .json(&request_params)
        .send()
        .map_err(error)
        .unwrap()
        .json()
        .map_err(error)
        .unwrap();

    println!("Repository Created: {}", res.clone_url);
    match options.mode {
        GitMode::Create => {}
        GitMode::Clone => {
            let repo_dir = git::clone(&res.clone_url, dir).map_err(error).unwrap();
            println!("Cloned into: {}", repo_dir);
        }
        GitMode::Remote => {
            let repo_dir = git::remotes(&res.clone_url, dir).map_err(error).unwrap();
            println!("Updated remotes for: {}", repo_dir);
        }
        GitMode::Push => {
            let repo_dir = git::remotes(&res.clone_url, dir).map_err(error).unwrap();
            println!("Updated remotes for: {}", repo_dir);
            let repo_dir = git::push(dir, user, pass).map_err(error).unwrap();
            println!("Pushed repository: {}", repo_dir);
        }
        GitMode::Rebase => {
            unimplemented!();
        }
    }
}

fn error<E>(err: E) -> !
    where E: std::error::Error
{
    error!("Error: {:?}", err);
    println!("Error: {}", err.description());
    std::process::exit(1)
}

fn prompt_create_params(editor: &str, options: &CreateRequest) -> Result<Option<CreateRequest>> {
    let mut tmp_file = try!(NamedTempFile::new());
    let _ = write!(tmp_file, "{}", options.to_template());
    let _ = tmp_file.sync_all();
    let path = try!(tmp_file.path().to_str().ok_or(Error::InvalidTargetDir)).to_string();
    {
        let _ = try!(tmp_file.persist(&path));
    }
    let (tx, rx) = channel();
    let closed = Arc::new(AtomicBool::new(false));
    let written = Arc::new(AtomicBool::new(false));

    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx));
    {
        let closed = closed.clone();
        let written = written.clone();
        let path = path.clone();
        thread::spawn(move || -> Result<()> {
            try!(watcher.watch(path));
            loop {
                if closed.load(Ordering::Relaxed) {
                    return Ok(());
                }
                match rx.try_recv() {
                    Err(TryRecvError::Disconnected) => return Ok(()),
                    Err(TryRecvError::Empty) => { }
                    Ok(notify::Event { op: Ok(notify::op::WRITE), .. }) => {
                        written.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        });
    }

    let status = try!(Command::new(editor).arg(&path).status());
    {
        closed.store(true, Ordering::Relaxed);
    }

    if !status.success() || !written.load(Ordering::Relaxed) {
        return Ok(None);
    }
    let mut tmp_file = try!(File::open(&path));
    let mut text = String::new();
    let _ = tmp_file.read_to_string(&mut text);
    try!(remove_file(&path));
    Ok(Some(try!(CreateRequest::from_template(&*text))))
}
