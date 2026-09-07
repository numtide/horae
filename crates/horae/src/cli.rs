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
            .unwrap_or_else(|| Commands::Serve(ServeArgs::default()))
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
    /// Populate the database with demo data (safe to re-run)
    Seed,
}

#[derive(Parser, Debug, Clone, Default)]
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
