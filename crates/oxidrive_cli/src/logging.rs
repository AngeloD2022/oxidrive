use indicatif::MultiProgress;
use std::sync::OnceLock;

static GLOBAL_PROGRESS: OnceLock<MultiProgress> = OnceLock::new();

pub fn set_global_progress(mp: MultiProgress) {
    let _ = GLOBAL_PROGRESS.set(mp);
}

pub fn log_step(subsystem: &str, message: impl AsRef<str>) {
    let now = chrono::Local::now();
    let timestamp = now.format("%M:%S%.3f").to_string();

    let output = format!(
        "{:<11} {:<6} {:<13} {}",
        timestamp,
        "INFO",
        subsystem.to_uppercase(),
        message.as_ref(),
    );

    if let Some(mp) = GLOBAL_PROGRESS.get() {
        let _ = mp.println(output);
    } else {
        println!("{}", output);
    }
}

#[macro_export]
macro_rules! log_info {
    ($subsys:expr, $($arg:tt)*) => {
        $crate::logging::log_step($subsys, format!($($arg)*))
    };
}
