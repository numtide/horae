use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "horae", about = "Horae time tracking server", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    /// Returns the resolved command, defaulting to `serve` when none is given.
    /// This lets `dx serve` launch the binary without arguments.
    pub fn command(self) -> Commands {
        self.command
            .unwrap_or_else(|| Commands::Serve(ServeArgs::parse_from(["horae"])))
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the HTTP server
    Serve(ServeArgs),
    /// Bootstrap a fresh installation: one organization and one admin user
    Init {
        /// Name of the organization to create
        #[arg(long)]
        org_name: String,
        /// Email address of the first admin user
        #[arg(long)]
        admin_email: String,
        /// Display name of the first admin user
        #[arg(long)]
        admin_name: String,
    },
    /// Database migration commands
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },
    /// User management commands
    User {
        #[command(subcommand)]
        action: UserAction,
    },
    /// Initialize an empty database with demo data; leave an existing demo unchanged
    Seed,
}

#[derive(Parser, Debug, Clone)]
pub struct ServeArgs {
    /// Host address to bind to
    #[arg(long, env = "HORAE_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(long, env = "HORAE_PORT", default_value_t = 3000)]
    pub port: u16,
}

#[derive(Subcommand, Debug)]
pub enum MigrateAction {
    /// Run pending migrations (default)
    Run,
    /// Drop every table and re-apply the migrations from scratch (dev only)
    Reset {
        /// Confirm destructive action
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum UserAction {
    /// Create a new user
    Create {
        #[arg(long)]
        email: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "member")]
        role: String,
    },
    /// List all users
    List,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_command_uses_the_same_defaults_and_environment_as_serve() {
        for (host, port) in [(None, None), (Some("localhost"), Some("4567"))] {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child.args(["--exact", "cli::tests::environment_probe", "--nocapture"]);
            child.env("HORAE_CLI_TEST", "compare");
            child.env_remove("HORAE_HOST").env_remove("HORAE_PORT");
            if let Some(host) = host {
                child.env("HORAE_HOST", host);
            }
            if let Some(port) = port {
                child.env("HORAE_PORT", port);
            }
            let output = child.output().unwrap();
            assert!(output.status.success(), "{output:?}");
        }
    }

    #[test]
    fn default_command_rejects_invalid_environment_ports() {
        for port in ["invalid", "65536", ""] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "cli::tests::environment_probe", "--nocapture"])
                .env("HORAE_CLI_TEST", "invalid")
                .env_remove("HORAE_HOST")
                .env("HORAE_PORT", port)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{output:?}");
        }
    }

    // A subprocess isolates Clap's environment reads from concurrent tests.
    #[test]
    fn environment_probe() {
        let Ok(mode) = std::env::var("HORAE_CLI_TEST") else {
            return;
        };
        let Commands::Serve(implicit) = Cli::parse_from(["horae"]).command() else {
            panic!("expected serve");
        };
        if mode == "compare" {
            let Commands::Serve(explicit) = Cli::parse_from(["horae", "serve"]).command() else {
                panic!("expected serve");
            };
            assert_eq!(
                (&implicit.host, implicit.port),
                (&explicit.host, explicit.port)
            );
            let Commands::Serve(flags) =
                Cli::parse_from(["horae", "serve", "--host", "::1", "--port", "5678"]).command()
            else {
                panic!("expected serve");
            };
            assert_eq!((flags.host.as_str(), flags.port), ("::1", 5678));
        }
    }
}
