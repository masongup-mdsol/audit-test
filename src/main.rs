#[macro_use]
extern crate serde_derive;
extern crate docopt;

use docopt::Docopt;
use std::path::PathBuf;

mod audit_creator;
mod audit_reader;
mod audit_sender;

const USAGE: &str = "
Audit Test

Usage:
    audit-test create-audits [--threads=<num>] [--audits=<num>] [--save-path=<path>] [-v]
    audit-test show-audit-size [-v]
    audit-test retrieve-audits [--what-uris=<num>] [--audit-id=<uuid>] [--load-path=<path>] [-v]
    audit-test send-audits
    audit-test test-crypto
    audit-test (-h | --help)
    audit-test --version

Options:
    -h --help               Show this screen
    -t --threads=<num>      Number of threads to use [default: 2]
    -a --audits=<num>       Number of audits to create per thread [default: 4]
    -u --what-uris=<num>    Number of audits for what_uris to fetch
    -p --save-path=<path>   Path to save the uuid output file
    --audit-id=<uuid>       Single audit ID to retrieve
    --load-path=<path>      Path to load a uuid retrieval file from
    -v --verbose            Output more details
    --version               Print the version
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_threads: i32,
    flag_audits: i32,
    flag_what_uris: Option<i32>,
    flag_verbose: bool,
    flag_audit_id: Option<String>,
    flag_save_path: Option<String>,
    flag_load_path: Option<String>,
    cmd_create_audits: bool,
    cmd_show_audit_size: bool,
    cmd_retrieve_audits: bool,
    cmd_send_audits: bool,
    cmd_test_crypto: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some("0.0.1".to_string())).help(true).deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_create_audits {
        let path_maybe = args.flag_save_path.and_then(|s| Some(PathBuf::from(&s)));
        audit_creator::create_audits_threaded(args.flag_threads, args.flag_audits, args.flag_verbose, path_maybe);
    }
    if args.cmd_show_audit_size {
        audit_creator::show_audit_size(args.flag_verbose);
    }
    if args.cmd_retrieve_audits {
        if let Some(what_uris) = args.flag_what_uris {
            audit_reader::retrieve_audits(what_uris.into(), args.flag_verbose);
        }
        if let Some(audit_id) = args.flag_audit_id {
            audit_reader::retrieve_audit_by_id(audit_id);
        }
        if let Some(load_path) = args.flag_load_path {
            audit_reader::retrieve_by_ids_from_file(load_path);
        }
    }
    if args.cmd_send_audits {
        audit_sender::send_audits();
    }
    if args.cmd_test_crypto {
        audit_sender::test_crypto();
    }
}

