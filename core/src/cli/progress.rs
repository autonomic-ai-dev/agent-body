use crate::ui::{ProgressMode, resolve_progress_mode};

pub const ENV: &str = "AUTONOMIC_PROGRESS";

pub fn apply_progress_env(mode: ProgressMode) {
    let value = match mode {
        ProgressMode::Auto => "auto",
        ProgressMode::Plain => "plain",
        ProgressMode::Quiet => "quiet",
    };
    unsafe {
        std::env::set_var(ENV, value);
    }
}

pub fn parse_progress_mode_str(raw: &str) -> Option<ProgressMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(ProgressMode::Auto),
        "plain" => Some(ProgressMode::Plain),
        "quiet" => Some(ProgressMode::Quiet),
        _ => None,
    }
}

/// Remove global `--progress` from argv (preserves argv[0]); apply env when set.
pub fn strip_progress_argv(args: Vec<String>) -> Vec<String> {
    if args.is_empty() {
        return args;
    }
    let mut mode = None::<ProgressMode>;
    let mut out = Vec::with_capacity(args.len());
    out.push(args[0].clone());
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--progress" {
            if let Some(v) = args.get(i + 1).and_then(|s| parse_progress_mode_str(s)) {
                mode = Some(v);
                i += 2;
                continue;
            }
        } else if let Some(rest) = args[i].strip_prefix("--progress=") {
            if let Some(m) = parse_progress_mode_str(rest) {
                mode = Some(m);
                i += 1;
                continue;
            }
        }
        out.push(args[i].clone());
        i += 1;
    }
    apply_progress_env(mode.unwrap_or_else(|| resolve_progress_mode(None)));
    out
}
