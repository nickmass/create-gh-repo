extern crate rpassword;

use git2::{Config, Repository, BranchType};
use error::{Error, Result};
use url::Url;
use std::path::Path;
use std::env;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GitMode {
    Create,
    Clone,
    Remote,
    Push,
    Rebase,
}

pub fn get_config_value(key: &str) -> Result<String> {
    let conf = try!(Config::open_default());
    conf.get_string(key).map_err(|e| e.into())
}

pub fn clone(repo_url: &str, target_dir: Option<&str>) -> Result<String> {
    let mut url = try!(Url::parse(repo_url));
    let _ = try!(url.set_host(Some("localhost")));
    let repo_path = target_dir.map(|x| x.to_string()).or_else(|| {
        match url.to_file_path()
            .ok()
            .map_or(None, |x| {
                x.file_stem()
                    .map(|x| x.to_string_lossy().into_owned())
            }) {
            Some(ref f) if f.len() > 0 => Some(f.clone()),
            _ => None,
        }
    });

    if repo_path.is_none() {
        return Err(Error::InvalidTargetDir);
    }

    let repo_path = repo_path.unwrap();
    let repo_path = Path::new(&repo_path);

    if !repo_path.exists() || (repo_path.is_dir() && repo_path.read_dir().unwrap().count() == 0) {
        let repo = try!(Repository::clone(repo_url, repo_path));
        get_repo_dir(&repo)
    } else {
        Err(Error::InvalidTargetDir)
    }
}

fn find_repository(target_dir: Option<&str>) -> Result<Repository> {
    let repo;
    if let Some(target_dir) = target_dir {
        repo = try!(Repository::open(target_dir));
    } else {
        let dir = try!(env::current_dir());
        repo = try!(Repository::discover(dir));
    }

    Ok(repo)
}

fn get_repo_dir(repo: &Repository) -> Result<String> {
    let path = try!(repo.workdir().ok_or(Error::RepositoryBare));
    Ok(path.to_string_lossy().to_owned().to_string())
}

pub fn get_repo_name(target_dir: Option<&str>) -> Result<String> {
    let repo = try!(find_repository(target_dir));
    let path = try!(repo.workdir().ok_or(Error::RepositoryBare));
    path.file_name()
        .ok_or(Error::RepositoryBare)
        .map(|x| x.to_string_lossy().to_owned().to_string())
}

fn set_upstream(repo: &mut Repository, local_branch: &str, remote_branch: &str) -> Result<()> {
    let mut master = try!(repo.find_branch(local_branch, BranchType::Local));
    master.set_upstream(Some(remote_branch)).map_err(|x| x.into())
}

pub fn remotes(repo_url: &str, target_dir: Option<&str>) -> Result<String> {
    let repo = try!(find_repository(target_dir));
    {
        let remote = repo.find_remote("origin").ok();
        let mut remote = if let Some(remote) = remote {
            let _ = try!(repo.remote_set_url("origin", repo_url));
            remote
        } else {
            try!(repo.remote("origin", repo_url))
        };
        let _ = try!(remote.fetch(&[], None, None));
    }
    let mut repo = repo;
    set_upstream(&mut repo, "master", "origin/master").ok();
    get_repo_dir(&repo)
}

pub fn push(target_dir: Option<&str>,
            username: Option<&str>,
            password: Option<&str>)
            -> Result<String> {
    use git2::{PushOptions, RemoteCallbacks, Cred};
    use std::io::stdin;
    let repo = try!(find_repository(target_dir));
    {
        let mut remote = try!(repo.find_remote("origin"));

        let mut cbs = RemoteCallbacks::new();
        cbs.credentials(|_, _, _| {
            // url, username from url, allowed cred types
            let username = if username.is_none() {
                let mut buf = String::new();
                let _ = stdin().read_line(&mut buf);
                buf
            } else {
                username.unwrap().clone().to_string()
            };

            let password = if password.is_none() {
                println!("Password: ");
                rpassword::read_password().unwrap()
            } else {
                password.unwrap().clone().to_string()
            };
            Cred::userpass_plaintext(username.trim(), &*password)
        });
        cbs.transfer_progress(|_| true);
        cbs.sideband_progress(|_| true);
        cbs.update_tips(|_, _, _| true);
        cbs.certificate_check(|_, _| true);

        try!(remote.push(&["refs/heads/master:refs/heads/master"],
                         Some(PushOptions::new().remote_callbacks(cbs))));
    }
    let mut repo = repo;
    try!(set_upstream(&mut repo, "master", "origin/master"));
    get_repo_dir(&repo)
}
