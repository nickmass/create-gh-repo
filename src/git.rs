use git2::Repository;
use error::{Error, Result};
use url::Url;
use std::path::Path;

pub enum GitMode {
    Create,
    Clone,
    Remotes,
    Push,
    Rebase,
}

pub fn clone(repo_url: &str, target_dir: Option<&str>) -> Result<String> {
    let mut url = try!(Url::parse(repo_url));
    let _ = try!(url.set_host(Some("localhost")));
    let repo_path = target_dir.map(|x| x.to_string()).or_else(|| {
        match url
            .to_file_path().ok()
            .map_or(None, |x| x.file_stem()
                    .map(|x| x.to_string_lossy().into_owned())) {
            Some(ref f) if f.len() > 0 => Some(f.clone()),
            _ => None,
        }
    });

    if repo_path.is_none() { return Err(Error::InvalidTargetDir); }
    
    let repo_path = repo_path.unwrap();
    let repo_path = Path::new(&repo_path);

    if !repo_path.exists() ||
        (repo_path.is_dir() &&
        repo_path.read_dir().unwrap().count() == 0) {
        let _ = try!(Repository::clone(repo_url, repo_path));
        Ok(repo_path.to_string_lossy().into_owned())
    } else {
        Err(Error::InvalidTargetDir)
    }
}
