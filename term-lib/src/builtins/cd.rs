use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use crate::eval::EvalContext;
use crate::eval::ExecResponse;
use crate::fs::AsyncifyFileSystem;
use crate::stdio::*;

#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

pub(super) fn cd(
    args: &[String],
    mut ctx: EvalContext,
    mut stdio: Stdio,
) -> Pin<Box<dyn Future<Output = ExecResponse> + Send>> {
    if args.len() > 2 {
        return Box::pin(async move {
            let _ = stdio
                .stderr
                .write(format!("cd: too many arguments\r\n").as_bytes())
                .await;
            ExecResponse::Immediate(ctx, 0)
        });
    }

    let mut print_path = false;
    let mut dir = if args.len() == 1 {
        home(&ctx)
    } else if args[1] == "-" {
        if let Some(v) = ctx.env.get("OLDPWD") {
            print_path = true;
            v
        } else {
            return Box::pin(async move {
                let _ = stdio
                    .stderr
                    .write(format!("cd: -: OLDPWD not set\r\n").as_bytes())
                    .await;
                ExecResponse::Immediate(ctx, 0)
            });
        }
    } else {
        let mut dir = args[1].clone();
        if dir.starts_with("/") == false {
            dir.insert_str(0, current(&ctx).as_str());
        }

        canonicalize(dir.as_str())
    };

    if dir.ends_with("/") == false {
        dir += "/";
    }

    Box::pin(async move {
        if AsyncifyFileSystem::new(ctx.root.clone())
            .read_dir(Path::new(dir.as_str()))
            .await
            .is_err()
        {
            let _ = stdio
                .stderr
                .write(format!("cd: {}: No such directory\r\n", dir).as_bytes())
                .await;
            return ExecResponse::Immediate(ctx, 0);
        }

        ctx.env.set_var("OLDPWD", current(&ctx));
        set_current(&mut ctx, dir.as_str());
        ctx.env.set_var("PWD", dir.clone());

        if print_path {
            let _ = stdio.stdout.write(format!("{}\r\n", dir).as_bytes()).await;
        }
        ExecResponse::Immediate(ctx, 0)
    })
}

fn canonicalize(path: &str) -> String {
    let mut ret = String::with_capacity(path.len());

    let mut comps = Vec::new();
    for comp in path.split("/") {
        if comp.len() <= 0 {
            continue;
        }
        if comp == "." {
            continue;
        };
        if comp == ".." {
            if comps.len() > 0 {
                comps.remove(comps.len() - 1);
            }
            continue;
        }
        comps.push(comp);
    }

    ret += "/";
    for comp in comps {
        if ret.ends_with("/") == false {
            ret += "/";
        }
        ret += comp;
    }
    ret
}

fn home(ctx: &EvalContext) -> String {
    ctx.env.get("HOME").unwrap_or_else(|| String::from("/"))
}

fn current(ctx: &EvalContext) -> String {
    ctx.working_dir.clone()
}

fn set_current(ctx: &mut EvalContext, path: &str) {
    ctx.working_dir = path.to_string();
}
