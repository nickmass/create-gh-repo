#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde_json;
extern crate tempfile;
extern crate hyper;
extern crate git2;
extern crate url;
#[macro_use]
extern crate clap;

mod cli;
use cli as Cli;
use std::env;
use std::process::Command;
use std::io::{Write, Read, Seek, SeekFrom};
use std::path::Path;

use tempfile::NamedTempFile;
use serde_json as json;
use hyper::client::{Client, RedirectPolicy};
use hyper::header::{Connection, Authorization, Basic, UserAgent};
use hyper::status::StatusCode;
use url::Url;
use git2::Repository;

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
            name: "repo-name".into(),
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
    let editor = env::var("EDITOR").unwrap_or("".into());
    let username = env::var("GITHUB_USERNAME").unwrap_or("".into());
    let token  = env::var("GITHUB_TOKEN").unwrap_or("".into());
    let password = env::var("GITHUB_PASSWORD").unwrap_or("".into());
  
    let matches = Cli::build_cli(&editor, &username, &password, &token).get_matches();

    let username = matches.value_of("username").unwrap();
    let token = matches.value_of("auth").unwrap();
    let editor = matches.value_of("editor").unwrap();

    let mut tmp_file = NamedTempFile::new().expect("Error creating temporary file");
    let _ = write!(tmp_file, "{}", template_text());
    let _ = tmp_file.sync_all();
    let path = tmp_file.path().to_str().unwrap();
    match Command::new(editor)
                    .arg(path)
                    .status() {
        Ok(status) if !status.success() => { return; }
        Err(e) => { panic!("{}", e); }
        _ => {}

    }

    let mut tmp_file = tmp_file.reopen().unwrap();
    let _ = tmp_file.seek(SeekFrom::Start(0));
    let mut text = String::new();
    let _ = tmp_file.read_to_string(&mut text);
    let json: CreateRequest = json::from_str(&*text).expect("Unable to parse repository parameters");

    let api_url = "https://api.github.com/user/repos";

    let mut client = match env::var("HTTP_PROXY") {
        Ok(mut proxy) => {
            let mut port = 80;
            if let Some(colon) = proxy.rfind(':') {
                port = proxy[colon + 1..].parse().expect("$HTTP_PROXY is invalid");
                proxy.truncate(colon);
            }
            Client::with_http_proxy(proxy, port)
        },
        _ => Client::new()
    };
    
    client.set_redirect_policy(RedirectPolicy::FollowAll);
    let mut res = client
        .post(&*api_url)
        .header(Connection::close())
        .header(Authorization(Basic { username: username.to_string(), password: Some(token.to_string()) }))
        .header(UserAgent("create-gh-repo".into()))
        .body(json::to_string(&json).unwrap().as_bytes())
        .send().unwrap();
   

    let mut res_body = String::new();
    let _ = res.read_to_string(&mut res_body);
    println!("{}", res.status);
    println!("{}", res_body);
    if res.status == StatusCode::Created {
        let res: CreateResponse = json::from_str(&*res_body).unwrap();
        println!("{}", "Repository Created:");
        println!("{}", res.clone_url);

        let repo_path = match env::args().skip(1).next() {
            Some(arg) => arg,
            None => {
                let url = Url::parse(&*res.clone_url).unwrap();
                let target = match url.path_segments() {
                    Some(segs) => segs.rev().next(),
                    None => panic!("No repository name supplied"),
                };

                let target = match target {
                    Some(target) if target != "" => target,
                    _ => panic!("No repository name supplied"),
                };

                match Path::new(target).file_stem() {
                    Some(target) => target.to_string_lossy().into_owned(),
                    None => panic!("No repository name supplied"),
                }
            }
        };
        let repo_path = Path::new(&*repo_path);

        if !repo_path.exists() ||
            (repo_path.is_dir() &&
            repo_path.read_dir().unwrap().count() == 0) {
            match Repository::clone(&*res.clone_url, repo_path) {
                Err(e) => panic!("Failed to clone repository: {}", e),
                _ => println!("Repository cloned into: {}", repo_path.to_string_lossy()),
            }
        } else {
            println!("Path: {}", repo_path.to_string_lossy());
        }
    }
}

fn template_text() -> String {
    let json = json::to_string_pretty(&CreateRequest::default()).unwrap();
    json
}
