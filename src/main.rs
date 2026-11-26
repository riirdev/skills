mod cli;
mod diff_run;
mod installer;
mod json;
mod ledger;
mod sha256;
mod targets;
mod unsafe_audit;
mod validate;

fn main() {
    std::process::exit(cli::run());
}
