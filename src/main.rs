#[macro_use]
extern crate serde_derive;
extern crate docopt;

use docopt::Docopt;

mod audit_creator;
mod audit_reader;

const USAGE: &str = "
Audit Test

Usage:
    audit-test create-audits [--threads=<num>] [--audits=<num>] [-v]
    audit-test show-audit-size [-v]
    audit-test retrieve-audits [--what-uris=<num>] [-v]
    audit-test (-h | --help)
    audit-test --version

Options:
    -h --help               Show this screen
    -t --threads=<num>      Number of threads to use [default: 2]
    -a --audits=<num>       Number of audits to create per thread [default: 4]
    -u --what-uris=<num>    Number of audits for what_uris to fetch [default: 10]
    -v --verbose            Output more details
    --version               Print the version
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_threads: i32,
    flag_audits: i32,
    flag_what_uris: i32,
    flag_verbose: bool,
    cmd_create_audits: bool,
    cmd_show_audit_size: bool,
    cmd_retrieve_audits: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some("0.0.1".to_string())).help(true).deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_create_audits {
        audit_creator::create_audits_threaded(args.flag_threads, args.flag_audits, args.flag_verbose);
    }
    if args.cmd_show_audit_size {
        audit_creator::show_audit_size(args.flag_verbose);
    }
    if args.cmd_retrieve_audits {
        audit_reader::retrieve_audits(args.flag_what_uris.into(), args.flag_verbose);
    }
}

