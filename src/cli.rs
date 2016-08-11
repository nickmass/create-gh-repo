use error::{Error, Result};
use git;
use git::GitMode;

use std::env;
use clap::{Arg, ArgGroup, App};

pub fn build_cli<'a>() -> App<'a, 'a> {
    App::new("Create GitHub Repositories")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Allows you to create new repositories on GitHub from the command line")
        .arg(Arg::with_name("username")
            .short("u")
            .long("user")
            .takes_value(true)
            .help("Your GitHub account username"))
        .arg(Arg::with_name("token")
            .short("t")
            .long("token")
            .takes_value(true)
            .conflicts_with("username")
            .help("A Personal Token for your GitHub account with the 'public_repo' permission"))
        .arg(Arg::with_name("password")
            .short("p")
            .long("password")
            .requires("username")
            .takes_value(true)
            .help("The password to your GitHub account"))
        .group(ArgGroup::with_name("auth").args(&["token", "password"]))
        .arg(Arg::with_name("editor")
            .short("e")
            .long("editor")
            .takes_value(true)
            .help("The command to run to edit the repository manifest"))
        .arg(Arg::with_name("directory")
            .help("Sets an optional target directory for git operations")
            .index(2))
        .arg(Arg::with_name("mode")
            .index(1)
            .possible_values(&["create", "clone", "remote", "push"])
            .default_value("clone")
            .required(true))
        .after_help("NOTES:{n}<username>, <token>, and <password> may alternatively be supplied \
                     by setting the GITHUB_USERNAME, GITHUB_TOKEN, or GITHUB_PASSWORD environment \
                     variables")
}

pub struct CommandOptions {
    pub editor: String,
    pub auth: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub mode: GitMode,
    pub directory: Option<String>,
}

pub struct CommandOptionsBuilder {
    editor: Option<String>,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
    directory: Option<String>,
    mode: Option<GitMode>,
    token_auth: Option<bool>,
}

impl CommandOptionsBuilder {
    pub fn new() -> CommandOptionsBuilder {
        CommandOptionsBuilder {
            editor: git::get_config_value("core.editor").or(env::var("EDITOR")).ok(),
            username: env::var("GITHUB_USERNAME").ok(),
            password: env::var("GITHUB_PASSWORD").ok(),
            token: env::var("GITHUB_TOKEN").ok(),
            directory: None,
            mode: None,
            token_auth: None,
        }
    }

    pub fn editor<S>(&mut self, editor: S) -> &mut Self
        where S: Into<String>
    {
        self.editor = Some(editor.into());
        self
    }

    pub fn username<S>(&mut self, username: S) -> &mut Self
        where S: Into<String>
    {
        self.username = Some(username.into());
        self
    }

    pub fn password<S>(&mut self, password: S) -> &mut Self
        where S: Into<String>
    {
        self.password = Some(password.into());
        self.token_auth = Some(false);
        self
    }

    pub fn token<S>(&mut self, token: S) -> &mut Self
        where S: Into<String>
    {
        self.token = Some(token.into());
        self.token_auth = Some(true);
        self
    }

    pub fn directory<S>(&mut self, directory: S) -> &mut Self
        where S: Into<String>
    {
        self.directory = Some(directory.into());
        self
    }

    pub fn mode(&mut self, mode: GitMode) -> &mut Self {
        self.mode = Some(mode);
        self
    }

    pub fn build(self) -> Result<CommandOptions> {
        let auth = match self.token_auth {
            None => {
                if self.token.is_some() {
                    self.token
                } else if self.password.is_some() && self.username.is_some() {
                    Some(format!("{}:{}",
                                 self.username.as_ref().unwrap(),
                                 self.password.as_ref().unwrap()))
                } else {
                    None
                }
            }
            Some(true) => self.token,
            Some(false) => {
                if self.password.is_some() && self.username.is_some() {
                    Some(format!("{}:{}",
                                 self.username.as_ref().unwrap(),
                                 self.password.as_ref().unwrap()))
                } else {
                    None
                }
            }
        };

        let auth = try!(auth.ok_or(Error::MissingParameter("authentication".into())));
        let editor = try!(self.editor.ok_or(Error::MissingParameter("editor".into())));
        let mode = try!(self.mode.ok_or(Error::MissingParameter("mode".into())));

        Ok(CommandOptions {
            editor: editor,
            auth: auth,
            username: self.username,
            password: self.password,
            directory: self.directory,
            mode: mode,
        })
    }
}

pub fn get_options(args: Option<Vec<&str>>) -> Result<CommandOptions> {
    let matches = if let Some(args) = args {
        build_cli().get_matches_from(args)
    } else {
        build_cli().get_matches()
    };

    let mode = match matches.value_of("mode") {
        Some("create") => GitMode::Create,
        Some("clone") => GitMode::Clone,
        Some("remote") => GitMode::Remote,
        Some("push") => GitMode::Push,
        Some("rebase") => GitMode::Rebase,
        _ => GitMode::Clone,
    };

    let mut builder = CommandOptionsBuilder::new();

    if let Some(editor) = matches.value_of("editor") {
        builder.editor(editor);
    }
    if let Some(username) = matches.value_of("username") {
        builder.username(username);
    }
    if let Some(password) = matches.value_of("password") {
        builder.password(password);
    }
    if let Some(token) = matches.value_of("token") {
        builder.token(token);
    }
    if let Some(directory) = matches.value_of("directory") {
        builder.directory(directory);
    }
    builder.mode(mode);
    builder.build()
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use git::GitMode;

    #[test]
    fn display_help() {
        build_cli().print_help().unwrap();
        println!("");
    }

    fn clear_vars() {
        env::remove_var("EDITOR");
        env::remove_var("GITHUB_USERNAME");
        env::remove_var("GITHUB_PASSWORD");
        env::remove_var("GITHUB_TOKEN");
    }

    #[test]
    fn load_options_from_env() {
        clear_vars();//TODO Inject env vars intp cli function
        ::std::thread::sleep(::std::time::Duration::from_millis(1000));
        // Very hacky, gets around parallel tests for now
        env::set_var("GITHUB_USERNAME", "user");
        env::set_var("GITHUB_PASSWORD", "pass");
        env::set_var("EDITOR", "vim");

        let opts = get_options(None).unwrap();
        assert_eq!(opts.username, Some("user".to_string()));
        assert_eq!(opts.password, Some("pass".to_string()));
        assert_eq!(opts.auth, "user:pass".to_string());
        assert_eq!(opts.mode, GitMode::Clone);

        clear_vars();
        env::set_var("GITHUB_USERNAME", "user");
        env::set_var("GITHUB_PASSWORD", "pass");
        env::set_var("GITHUB_TOKEN", "token");
        env::set_var("EDITOR", "vim");

        let opts = get_options(None).unwrap();
        assert_eq!(opts.username, Some("user".to_string()));
        assert_eq!(opts.password, Some("pass".to_string()));
        assert_eq!(opts.auth, "token".to_string());
        assert_eq!(opts.mode, GitMode::Clone);
    }

    #[test]
    fn set_options() {
        clear_vars();
        let opts = vec!["create_gh_repo", "-e=vim", "-p=pass", "-u=user", "create", "somedir"];
        let opts = get_options(Some(opts)).unwrap();
        assert_eq!(opts.username, Some("user".to_string()));
        assert_eq!(opts.password, Some("pass".to_string()));
        assert_eq!(opts.auth, "user:pass".to_string());
        assert_eq!(opts.mode, GitMode::Create);
        assert_eq!(opts.editor, "vim".to_string());
        assert_eq!(opts.directory, Some("somedir".to_string()));

        let opts = vec!["create_gh_repo",
                        "--editor=vim",
                        "--password=pass",
                        "--user=user",
                        "create",
                        "somedir"];
        let opts = get_options(Some(opts)).unwrap();
        assert_eq!(opts.username, Some("user".to_string()));
        assert_eq!(opts.password, Some("pass".to_string()));
        assert_eq!(opts.auth, "user:pass".to_string());
        assert_eq!(opts.mode, GitMode::Create);
        assert_eq!(opts.editor, "vim".to_string());
        assert_eq!(opts.directory, Some("somedir".to_string()));

        let opts = vec!["create_gh_repo", "--editor=vim", "--token=token", "create", "somedir"];
        let opts = get_options(Some(opts)).unwrap();
        assert_eq!(opts.username, None);
        assert_eq!(opts.password, None);
        assert_eq!(opts.auth, "token".to_string());
        assert_eq!(opts.mode, GitMode::Create);
        assert_eq!(opts.editor, "vim".to_string());
        assert_eq!(opts.directory, Some("somedir".to_string()));
    }
}
