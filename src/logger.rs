pub enum LogLevel {
    Error,
    Warn,
    Info,
    Success
}

// Later turn this into a macro
pub fn log(level: LogLevel, msg: &str) {
    match level {
        LogLevel::Error => {
            eprintln!("[\x1b[91m-\x1b[0m] {}", msg);
        },

        LogLevel::Warn => {
            eprintln!("[\x1b[93m!\x1b[0m] {}", msg);
        },
        LogLevel::Info => {
            println!("[\x1b[94m*\x1b[0m] {}", msg);
        },

        LogLevel::Success => {
            eprintln!("[\x1b[92m+\x1b[0m] {}", msg);
        }
    }
}
