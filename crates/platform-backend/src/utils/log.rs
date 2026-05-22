// 6.4 Logging Utility mit automatischem File:Line und Timestamp

use std::time::SystemTime;

pub enum Level {
    Info,
    Ok,
    Warn,
    Error,
}

impl Level {
    fn label(&self) -> &str {
        match self {
            Level::Info => "INFO",
            Level::Ok => " OK ",
            Level::Warn => "WARN",
            Level::Error => " ERR",
        }
    }

    fn color(&self) -> &str {
        match self {
            Level::Info => "36",
            Level::Ok => "32",
            Level::Warn => "33",
            Level::Error => "31",
        }
    }
}

fn timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn _emit(level: Level, file: &str, line: u32, msg: &str) {
    let short_file = file.split('/').last().unwrap_or(file);
    let output = format!(
        "\x1b[2m{}\x1b[0m \x1b[{}m[{}]\x1b[0m \x1b[2m{}:{}\x1b[0m {}",
        timestamp(),
        level.color(),
        level.label(),
        short_file,
        line,
        msg
    );
    match level {
        Level::Error => eprintln!("{}", output),
        _ => println!("{}", output),
    }
}

#[macro_export]
macro_rules! elog {
    ($level:ident, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::utils::log::_emit(
            $crate::utils::log::Level::$level,
            file!(),
            line!(),
            &msg,
        );
    }};
}
