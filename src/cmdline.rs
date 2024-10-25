use crate::uri_meta::Source;
use anyhow::Context;
use regex::Regex;
use std::env;
use std::io::prelude::Write;
use termion::input::TermRead;

pub fn parse_source(input: &str) -> Source {
    let mut input = input.to_string();
    /* A gitlab uri should be on the pattern
     * gitlab.<optional selfhosted org>.<tld>/<org>/<group>/../<repo>
     *
     * Though it's reasonable that a user delimits the remote with ':'
     * as done in the git protocol, such as
     * gitlab.com:<org>/<group>/../<repo>.git
     *
     * So we'll allow ':' delimitation and just replace it with '/'.
     *
     * There could definetly be bugs in the pattern here
     */
    let webpattern = Regex::new(r"gitlab.*\.[a-z, A-Z, 0-9]*(:|\/)").unwrap();
    if webpattern.is_match(&input) {
        // if we get something like gitlab.com:org/group...
        input = input.replacen(":", "/", 1);
        return Source::Web(input);
    }
    // we'll try to replace any '~' with HOME
    let Some(home) = env::vars().find(|(k, _)| k == "HOME") else {
        return Source::Disk(input.to_string());
    };
    Source::Disk(input.replace("~", &home.1))
}

pub fn select_option(msg: &str, options: &[String]) -> anyhow::Result<String> {
    if options.is_empty() {
        anyhow::bail!("no options to choose from")
    }

    let selected = {
        let mut err = std::io::stderr();
        loop {
            for (i, k) in options.iter().enumerate() {
                writeln!(err, "[{}]: {k}", i + 1)?;
            }

            write!(err, "{}", msg)?;
            err.flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            let Ok(selected) = input.trim().parse::<usize>() else {
                continue;
            };
            if selected > options.len() {
                continue;
            }
            break selected;
        }
    };

    Ok(options[selected - 1].clone())
}

pub fn input_with_prompt(prompt: &str) -> anyhow::Result<String> {
    let mut err = std::io::stderr();
    write!(err, "{}", prompt)?;
    err.flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn hidden_input_with_prompt(prompt: &str) -> anyhow::Result<String> {
    let mut err = std::io::stderr();
    write!(err, "{}", prompt)?;
    err.flush()?;
    let input = std::io::stdin()
        .read_passwd(&mut err)?
        .context("failed to read stdin")?;
    writeln!(err, "*********")?;
    err.flush()?;
    return Ok(input.trim().to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_source() {
        let home = env::vars().find(|(k, _)| k == "HOME").unwrap().1;
        assert_eq!(
            parse_source("gitlab.com/org/foo"),
            Source::Web("gitlab.com/org/foo".to_string()),
            "failed to parse web url"
        );
        assert_eq!(
            parse_source("gitlab.com:org/foo"),
            Source::Web("gitlab.com/org/foo".to_string()),
            "failed to parse web url"
        );
        assert_eq!(
            parse_source("gitlab.com:org/foo.git"),
            Source::Web("gitlab.com/org/foo.git".to_string()),
            "failed to parse web url"
        );
        assert_eq!(
            parse_source("~/git/foo"),
            Source::Disk(home + "/git/foo"),
            "failed to parse disk path"
        );
    }
}
