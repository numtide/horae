use clap::{Parser, Subcommand};

pub mod imports;

#[derive(Parser, Debug)]
#[command(name = "horae", about = "Horae time tracking server", version)]
pub struct Cli {
    #[command(flatten)]
    pub remote: RemoteOptions,
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
    /// Submit a durable Harvest import to a running server
    Import {
        #[command(subcommand)]
        action: ImportAction,
    },
    /// Inspect and control durable Harvest import jobs
    Jobs {
        #[command(subcommand)]
        action: JobAction,
    },
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

#[derive(clap::Args, Debug, Clone, Default)]
pub struct RemoteOptions {
    /// Private JSON file containing server_origin and the existing session_id
    #[arg(long, global = true, env = "HORAE_SESSION_FILE")]
    pub session_file: Option<std::path::PathBuf>,
    /// Emit a single versioned JSON result for a remote command
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct SubmitOptions {
    /// Preview without persisting imported data (default: commit)
    #[arg(long)]
    pub dry_run: bool,
    /// Reuse this UUIDv7 to resubmit identical input within 24 hours
    #[arg(long)]
    pub request_id: Option<uuid::Uuid>,
    #[command(flatten)]
    pub wait: WaitOptions,
}

#[derive(clap::Args, Debug)]
pub struct WaitOptions {
    /// Wait for the job's terminal outcome
    #[arg(long)]
    pub wait: bool,
    /// Maximum waiting time in seconds; timing out does not cancel the job
    #[arg(long, requires = "wait", value_parser = clap::value_parser!(u64).range(1..=86400))]
    pub timeout: Option<u64>,
}

#[derive(Subcommand, Debug)]
pub enum ImportAction {
    /// Import from the organization's existing Harvest connection
    HarvestApi {
        #[arg(long, conflicts_with = "incremental")]
        full: bool,
        #[arg(long)]
        incremental: bool,
        #[command(flatten)]
        options: SubmitOptions,
    },
    /// Upload a Harvest CSV (at most 50 MiB)
    HarvestCsv {
        file: std::path::PathBuf,
        #[command(flatten)]
        options: SubmitOptions,
    },
}

#[derive(Subcommand, Debug)]
pub enum JobAction {
    /// Inspect a job's current state
    Status { job_id: uuid::Uuid },
    /// List a bounded page of retained jobs
    List {
        #[arg(long)]
        before: Option<uuid::Uuid>,
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=100))]
        limit: u16,
    },
    /// Read a job's confirmed outcome summary
    Report { job_id: uuid::Uuid },
    /// Download all retained record errors to a file
    Errors {
        job_id: uuid::Uuid,
        #[arg(long)]
        output: std::path::PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Request cooperative cancellation
    Cancel { job_id: uuid::Uuid },
    /// Retry an eligible failed or cancelled job
    Retry {
        job_id: uuid::Uuid,
        #[command(flatten)]
        options: WaitOptions,
    },
    /// Wait for a job without changing its execution
    Wait {
        job_id: uuid::Uuid,
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..=86400))]
        timeout: Option<u64>,
    },
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
    fn remote_import_and_job_commands_accept_the_documented_arguments() {
        for args in [
            vec![
                "horae",
                "--session-file",
                "session.json",
                "import",
                "harvest-api",
                "--dry-run",
                "--full",
                "--wait",
                "--timeout",
                "120",
                "--json",
            ],
            vec![
                "horae",
                "import",
                "harvest-csv",
                "sample.csv",
                "--session-file",
                "session.json",
            ],
            vec![
                "horae",
                "jobs",
                "list",
                "--limit",
                "100",
                "--json",
                "--session-file",
                "session.json",
            ],
        ] {
            let parsed = Cli::try_parse_from(&args);
            assert!(parsed.is_ok(), "documented arguments {args:?}: {parsed:?}");
        }
    }

    #[test]
    fn remote_commands_reject_conflicting_or_unbounded_options() {
        for args in [
            vec!["horae", "import", "harvest-api", "--full", "--incremental"],
            vec!["horae", "import", "harvest-api", "--timeout", "10"],
            vec!["horae", "jobs", "list", "--limit", "101"],
            vec!["horae", "jobs", "list", "--limit", "0"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }

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
