# Create GitHub Repositories
[![License](https://img.shields.io/github/license/nickmass/create-gh-repo.svg)](https://raw.githubusercontent.com/nickmass/create-gh-repo/master/LICENSE) [![Build status](https://img.shields.io/travis/nickmass/create-gh-repo/master.svg)](https://travis-ci.org/nickmass/create-gh-repo) [![Build status](https://img.shields.io/appveyor/ci/nickmass/create-gh-repo/master.svg)](https://ci.appveyor.com/project/nickmass/create-gh-repo)

Create new Github repositories from the command line

```
Create GitHub Repositories 0.1.3
Nick Massey <nickmass@nickmass.com>
Allows you to create new repositories on GitHub from the command line

USAGE:
    create-gh-repo [OPTIONS] <mode> [ARGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --editor <editor>        The command to run to edit the repository manifest
    -p, --password <password>    The password to your GitHub account
    -t, --token <token>          A Personal Token for your GitHub account with the 'public_repo' permission
    -u, --user <username>        Your GitHub account username

ARGS:
    <mode>         Action taken after creating github repository [default: clone]  [values: create, clone, remote,
                   push]
    <directory>    Sets an optional target directory for git operations

SUBCOMMANDS:
    completions    Generate completion scripts for your shell
    help           Prints this message or the help of the given subcommand(s)

NOTES:
<username>, <token>, and <password> may alternatively be supplied by setting the GITHUB_USERNAME, GITHUB_TOKEN, or
GITHUB_PASSWORD environment variables
```
