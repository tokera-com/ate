#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use url::Url;
use ate::{prelude::*};
use ate_auth::prelude::*;
use clap::Parser;
use ate_auth::cmd::*;
use ate_auth::opt::*;
use ate_auth::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AteError>
{
    let opts: Opts = Opts::parse();

    ate::log_init(opts.verbose, opts.debug);

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::User(opts_user) => {
            main_opts_user(opts_user, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Group(opts_group) => {
            main_opts_group(opts_group, opts.token, opts.token_path, opts.auth, "Group").await?;
        },
        SubCommand::Token(opts_token) => {
            main_opts_token(opts_token, opts.token, opts.token_path, opts.auth, "Group").await?;
        }
    }

    // We are done
    Ok(())
}