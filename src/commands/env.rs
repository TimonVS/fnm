use super::command::Command;
use crate::config::FnmConfig;
use crate::fs::symlink_dir;
use crate::outln;
use crate::shell::{infer_shell, Shell, AVAILABLE_SHELLS};
use colored::Colorize;
use snafu::{OptionExt, Snafu};
use std::fmt::Debug;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Default)]
pub struct Env {
    /// The shell syntax to use. Infers when missing.
    #[structopt(long)]
    #[structopt(possible_values = AVAILABLE_SHELLS)]
    shell: Option<Box<dyn Shell>>,
    /// Deprecated. This is the default now.
    #[structopt(long, hidden = true)]
    multi: bool,
    /// Print the script to change Node versions every directory change
    #[structopt(long)]
    use_on_cd: bool,
}

fn generate_symlink_path(root: &std::path::Path) -> std::path::PathBuf {
    let temp_dir_name = format!(
        "fnm_multishell_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis(),
    );
    root.join(temp_dir_name)
}

fn make_symlink(config: &FnmConfig) -> std::path::PathBuf {
    let system_temp_dir = std::env::temp_dir();
    let mut temp_dir = generate_symlink_path(&system_temp_dir);

    while temp_dir.exists() {
        temp_dir = generate_symlink_path(&system_temp_dir);
    }

    symlink_dir(config.default_version_dir(), &temp_dir).expect("Can't create symlink!");
    temp_dir
}

impl Command for Env {
    type Error = Error;

    fn apply(self, config: &FnmConfig) -> Result<(), Self::Error> {
        if self.multi {
            outln!(config#Error, "{} {} is deprecated. This is now the default.", "warning:".yellow().bold(), "--multi".italic());
        }

        let shell: Box<dyn Shell> = self.shell.or_else(&infer_shell).context(CantInferShell)?;
        let multishell_path = make_symlink(&config);
        let binary_path = if cfg!(windows) {
            multishell_path.clone()
        } else {
            multishell_path.join("bin")
        };
        println!("{}", shell.path(&binary_path));
        println!(
            "{}",
            shell.set_env_var("FNM_MULTISHELL_PATH", multishell_path.to_str().unwrap())
        );
        println!(
            "{}",
            shell.set_env_var("FNM_DIR", config.base_dir_with_default().to_str().unwrap())
        );
        println!(
            "{}",
            shell.set_env_var("FNM_LOGLEVEL", config.log_level().clone().into())
        );
        println!(
            "{}",
            shell.set_env_var("FNM_NODE_DIST_MIRROR", config.node_dist_mirror.as_str())
        );
        if self.use_on_cd {
            println!("{}", shell.use_on_cd(&config));
        }
        Ok(())
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display(
        "{}\n{}\n{}\n{}",
        "Can't infer shell!",
        "fnm can't infer your shell based on the process tree.",
        "Maybe it is unsupported? we support the following shells:",
        shells_as_string()
    ))]
    CantInferShell,
}

fn shells_as_string() -> String {
    AVAILABLE_SHELLS
        .iter()
        .map(|x| format!("* {}", x))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke() {
        use crate::shell;
        let config = FnmConfig::default();
        let shell: Box<dyn Shell> = if cfg!(windows) {
            Box::from(shell::WindowsCmd)
        } else {
            Box::from(shell::Bash)
        };
        Env {
            shell: Some(shell),
            ..Env::default()
        }
        .call(config);
    }
}
