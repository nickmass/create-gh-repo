use error::{Error, Result};
use git::GitMode;

use std::env;
use clap::{Arg, ArgGroup, App};

pub fn build_cli<'a>() -> App<'a,'a> {
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
        .group(ArgGroup::with_name("auth")
               .args(&["token", "password"]))
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
             .possible_values(&["create", "clone", "remotes", "push", "rebase"])
             .default_value("clone")
             .required(true))
        .after_help("NOTES:{n}<username>, <token>, and <password> may alternatively be supplied by setting the GITHUB_USERNAME, GITHUB_TOKEN, or GITHUB_PASSWORD environment variables")
}

pub struct CommandOptions {
    pub editor: String,
    pub auth: String,
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
    token_auth: bool,
}

impl CommandOptionsBuilder {
    pub fn new() -> CommandOptionsBuilder {
        CommandOptionsBuilder {
            editor: env::var("EDITOR").ok(),
            username: env::var("GITHUB_USERNAME").ok(),
            password: env::var("GITHUB_PASSWORD").ok(),
            token: env::var("GITHUB_TOKEN").ok(),
            directory: None,
            mode: None,
            token_auth: true,
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
        self.token_auth = false;
        self
    }

    pub fn token<S>(&mut self, token: S) -> &mut Self
        where S: Into<String>
    {
        self.token = Some(token.into());
        self.token_auth = true;
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
        let auth = if self.token_auth {
            self.token
        } else if !self.token_auth && self.password.is_some() && self.username.is_some() {
            Some(format!("{}:{}", self.username.unwrap(), self.password.unwrap()))
        } else {
            None
        };

        let auth = try!(auth.ok_or(Error::MissingParameter("authentication".into())));
        let editor = try!(self.editor.ok_or(Error::MissingParameter("editor".into())));
        let mode = try!(self.mode.ok_or(Error::MissingParameter("mode".into())));

        Ok(CommandOptions {
            editor: editor,
            auth: auth,
            directory: self.directory,
            mode: mode,
        })
    }
}

pub fn get_options() -> Result<CommandOptions> {
    let matches = build_cli().get_matches();

    let mode = match matches.value_of("mode") {
        Some("create") => GitMode::Create,
        Some("clone") => GitMode::Clone,
        Some("remotes") => GitMode::Remotes,
        Some("push") => GitMode::Push,
        Some("rebase") => GitMode::Rebase,
        _ => GitMode::Clone,
    };

    let mut  builder = CommandOptionsBuilder::new();
       
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

