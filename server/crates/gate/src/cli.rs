use clap::Subcommand;

use crate::storage::{self, list_credentials};

#[derive(Subcommand)]
pub enum GateCommand {
    /// Manage invitation codes
    Invite {
        #[command(subcommand)]
        command: InviteCommand,
    },
    /// Manage WebAuthn credentials
    Credential {
        #[command(subcommand)]
        command: CredentialCommand,
    },
    /// Show gate module statistics
    Stats,
}

#[derive(Subcommand)]
pub enum InviteCommand {
    /// Create a one-time invitation code (auto-generated, shown once)
    Create {
        /// Family/group the user will belong to (required)
        #[arg(short, long)]
        family: String,
        /// Length of the auto-generated code (default: 8)
        #[arg(short = 'l', long, default_value = "8")]
        code_length: usize,
        /// Validity duration in seconds (default: 60, 0 = never expires)
        #[arg(long, default_value = "60")]
        ttl: u64,
    },
}

#[derive(Subcommand)]
pub enum CredentialCommand {
    /// List all credentials, optionally filtered by family
    List {
        #[arg(short, long)]
        family: Option<String>,
    },
    /// Disable a credential (revoke without deleting)
    Disable {
        #[arg(short, long)]
        id: String,
    },
    /// Permanently delete a credential
    Delete {
        #[arg(short, long)]
        id: String,
    },
}

pub fn handle(command: GateCommand) -> anyhow::Result<()> {
    match command {
        GateCommand::Invite { command } => handle_invite(command)?,
        GateCommand::Credential { command } => handle_credential(command)?,
        GateCommand::Stats => handle_stats()?,
    }
    Ok(())
}

fn handle_invite(cmd: InviteCommand) -> anyhow::Result<()> {
    match cmd {
        InviteCommand::Create {
            family,
            code_length,
            ttl,
        } => {
            let code = crate::api::cli_create_invite(&family, Some(code_length), Some(ttl))?;
            println!("Invite created for family \"{}\"", family);
            println!("Temp code: {}", code);
            if ttl > 0 {
                println!("Expires in: {} seconds", ttl);
            } else {
                println!("Expires: never");
            }
            println!("⚠  This code is shown ONLY once. Share it securely with the user.");
        }
    }
    Ok(())
}

fn handle_credential(cmd: CredentialCommand) -> anyhow::Result<()> {
    let db = utils::database::Db::new();
    match cmd {
        CredentialCommand::List { family } => {
            let creds = list_credentials(&db, family.as_deref())?;
            if creds.is_empty() {
                println!("No credentials found.");
            } else {
                println!("{:<36}  {:<16}  {:<16}  {:<8}  {:<20}", "ID", "NAME", "FAMILY", "ENABLED", "CREATED");
                println!("{}", "-".repeat(100));
                for (id, cred) in &creds {
                    println!(
                        "{:<36}  {:<16}  {:<16}  {:<8}  {:<20}",
                        id,
                        cred.name,
                        cred.family,
                        if cred.enabled { "yes" } else { "no" },
                        cred.created_at,
                    );
                }
            }
        }
        CredentialCommand::Disable { id } => {
            storage::disable_credential(&db, &id)?;
            db.flush()?;
            println!("Credential {} disabled.", id);
        }
        CredentialCommand::Delete { id } => {
            storage::delete_credential(&db, &id)?;
            db.flush()?;
            println!("Credential {} deleted.", id);
        }
    }
    Ok(())
}

fn handle_stats() -> anyhow::Result<()> {
    let db = utils::database::Db::new();
    let (total, enabled, families) = storage::stats(&db)?;
    println!("Credentials: {} total, {} enabled, {} disabled", total, enabled, total - enabled);
    println!("Families: {}", if families.is_empty() { "(none)".into() } else { families.join(", ") });
    Ok(())
}
