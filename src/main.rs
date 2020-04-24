#[macro_use]
extern crate serde_derive;

use std::path::PathBuf;
use structopt::StructOpt;
use uuid::Uuid;

mod audit_creator;
mod audit_reader;
mod audit_sender;

#[derive(StructOpt)]
#[structopt(name = "Audit Test")]
enum AppCommands {
    CreateAudits {
        #[structopt(short, long, default_value = "2")]
        threads: i32,
        #[structopt(short, long, default_value = "4")]
        audits: i32,
        #[structopt(short, long)]
        save_path: Option<PathBuf>,
        #[structopt(short, long)]
        verbose: bool,
    },
    ShowAuditSize {
        #[structopt(short, long)]
        verbose: bool,
    },
    RetrieveAudits {
        #[structopt(short, long)]
        what_uris: Option<i32>,
        #[structopt(short, long)]
        audit_id: Option<Uuid>,
        #[structopt(short, long)]
        load_path: Option<PathBuf>,
        #[structopt(short, long)]
        verbose: bool,
    },
    SendAudits,
    DaltonTest,
}

#[tokio::main]
async fn main() {
    match AppCommands::from_args() {
        AppCommands::SendAudits => audit_sender::send_audits().await,
        AppCommands::DaltonTest => audit_sender::dalton_test().await,
        AppCommands::ShowAuditSize { verbose } => audit_creator::show_audit_size(verbose),
        AppCommands::CreateAudits {
            threads,
            audits,
            save_path,
            verbose,
        } => audit_creator::create_audits_threaded(threads, audits, verbose, save_path).await,
        AppCommands::RetrieveAudits {
            what_uris,
            audit_id,
            load_path,
            verbose,
        } => {
            if let Some(path) = load_path {
                audit_reader::retrieve_by_ids_from_file(path).await;
            }
            if let Some(what_uris_real) = what_uris {
                audit_reader::retrieve_audits(what_uris_real.into(), verbose).await;
            }
            if let Some(audit_id_real) = audit_id {
                audit_reader::retrieve_audit_by_id(audit_id_real).await;
            }
        }
    }
}
