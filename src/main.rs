#[macro_use]
extern crate serde_derive;
extern crate docopt;

use docopt::Docopt;

mod audit_creator;
mod audit_reader;

const USAGE: &'static str = "
Audit Test

Usage:
    audit-test create-audits [--threads=<num>] [--audits=<num>]
    audit-test show-audit-size
    audit-test retrieve-audits [--what-uris=<num>]
    audit-test (-h | --help)

Options:
    -h --help            Show this screen
    --threads=<num>      Number of threads to use [default: 2]
    --audits=<num>       Number of audits to create per thread [default: 4]
    --what-uris=<num>    Number of audits for what_uris to fetch [default: 10]
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_threads: i32,
    flag_audits: i32,
    flag_what_uris: i32,
    cmd_create_audits: bool,
    cmd_show_audit_size: bool,
    cmd_retrieve_audits: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_create_audits {
        audit_creator::create_audits_threaded(args.flag_threads, args.flag_audits);
    }
    if args.cmd_show_audit_size {
        audit_creator::show_audit_size();
    }
    if args.cmd_retrieve_audits {
        audit_reader::retrieve_audits(args.flag_what_uris.into());
    }
}

