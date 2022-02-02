#![allow(unused_imports)]
use ate::prelude::*;
use error_chain::bail;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;

use crate::error::*;
use crate::helper::*;
use crate::opt::*;
use crate::prelude::*;
use crate::request::*;

use super::*;

pub async fn gather_command(
    registry: &Registry,
    group: String,
    session: AteSessionInner,
    auth: Url,
) -> Result<AteSessionGroup, GatherError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Create the gather command
    let gather = GatherRequest {
        group: group.clone(),
        session,
    };

    // Attempt the gather request with a 10 second timeout
    let response: Result<GatherResponse, GatherFailed> = chain.invoke(gather).await?;
    let result = response?;
    Ok(result.authority)
}

pub async fn main_session_group(
    token_string: Option<String>,
    token_file_path: Option<String>,
    group: String,
    sudo: bool,
    code: Option<String>,
    auth_url: Option<url::Url>,
    hint_group: &str,
) -> Result<AteSessionGroup, GatherError> {
    main_session_group_ext(token_string, token_file_path, group, sudo, code, auth_url, hint_group, false).await
}

pub async fn main_session_group_ext(
    token_string: Option<String>,
    token_file_path: Option<String>,
    group: String,
    sudo: bool,
    code: Option<String>,
    auth_url: Option<url::Url>,
    hint_group: &str,
    save: bool,
) -> Result<AteSessionGroup, GatherError> {
    let session = main_session_start(token_string, token_file_path.clone(), auth_url.clone()).await?;

    let mut session = match session {
        AteSessionType::Group(a) => {
            if a.group.name == group {
                return Ok(a);
            }
            a.inner
        }
        AteSessionType::User(a) => AteSessionInner::User(a),
        AteSessionType::Sudo(a) => AteSessionInner::Sudo(a),
    };

    if sudo {
        session = match session {
            AteSessionInner::User(a) => {
                if let Some(auth) = auth_url.clone() {
                    AteSessionInner::Sudo(main_sudo(a, code, auth).await?)
                } else {
                    AteSessionInner::User(a)
                }
            }
            a => a,
        };
    }

    if save {
        if let Some(token_file_path) = token_file_path.clone() {
            let token = session_to_b64(session.clone().into())?;
            save_token(token, token_file_path)?;
        }
    }

    if let Some(auth) = auth_url {
        Ok(main_gather(Some(group), session, auth, hint_group).await?)
    } else {
        Ok(AteSessionGroup::new(session, group))
    }
}

pub async fn main_gather(
    group: Option<String>,
    session: AteSessionInner,
    auth: Url,
    hint_group: &str,
) -> Result<AteSessionGroup, GatherError> {
    let group = match group {
        Some(a) => a,
        None => {
            eprint!("{}: ", hint_group);
            stdout().lock().flush()?;
            let mut s = String::new();
            std::io::stdin()
                .read_line(&mut s)
                .expect(format!("Did not enter a valid {}", hint_group.to_lowercase()).as_str());
            s.trim().to_string()
        }
    };

    // Gather using the authentication server which will give us a new session with the extra tokens
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.cement();
    let session = gather_command(&registry, group, session, auth).await?;
    Ok(session)
}
