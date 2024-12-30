#![allow(clippy::too_many_arguments, clippy::to_string_trait_impl)]

mod cli;
mod gui;
mod lang;
mod media;
mod metadata;
mod path;
mod prelude;
mod resource;

#[cfg(test)]
mod testing;

use crate::{
    gui::Flags,
    prelude::{app_dir, CONFIG_DIR, VERSION},
};

/// The logger handle must be retained until the application closes.
/// https://docs.rs/flexi_logger/0.23.1/flexi_logger/error_info/index.html#write
fn prepare_logging() -> Result<flexi_logger::LoggerHandle, flexi_logger::FlexiLoggerError> {
    flexi_logger::Logger::try_with_env_or_str("madamiru=warn")
        .unwrap()
        .log_to_file(flexi_logger::FileSpec::default().directory(app_dir().as_std_path_buf().unwrap()))
        .write_mode(flexi_logger::WriteMode::BufferAndFlush)
        .rotate(
            flexi_logger::Criterion::Size(1024 * 1024 * 10),
            flexi_logger::Naming::Timestamps,
            flexi_logger::Cleanup::KeepLogFiles(4),
        )
        .use_utc()
        .format_for_files(|w, now, record| {
            write!(
                w,
                "[{}] {} [{}] {}",
                now.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args(),
            )
        })
        .start()
}

/// Based on: https://github.com/Traverse-Research/panic-log/blob/874a61b24a8bc8f9b07f9c26dc10b13cbc2622f9/src/lib.rs#L26
/// Modified to flush a provided log handle.
fn prepare_panic_hook(handle: Option<flexi_logger::LoggerHandle>) {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread_name = std::thread::current().name().unwrap_or("<unnamed thread>").to_owned();

        let location = if let Some(panic_location) = info.location() {
            format!(
                "{}:{}:{}",
                panic_location.file(),
                panic_location.line(),
                panic_location.column()
            )
        } else {
            "<unknown location>".to_owned()
        };
        let message = info.payload().downcast_ref::<&str>().unwrap_or(&"");

        let backtrace = std::backtrace::Backtrace::force_capture();

        log::error!("thread '{thread_name}' panicked at {location}:\n{message}\nstack backtrace:\n{backtrace}");

        if let Some(handle) = handle.clone() {
            handle.flush();
        }

        original_hook(info);
    }));
}

/// Detach the current process from its console on Windows.
///
/// ## Testing
/// This has several edge cases and has been the source of multiple bugs.
/// If you change this, be careful and make sure to test this matrix:
///
/// * Arguments:
///   * None (double click in Windows Explorer)
///   * None (from console)
///   * `--help` (has output, but before this function is called)
///   * `schema config` (has output, after this function is called)
/// * Consoles:
///   * Command Prompt
///   * PowerShell
///   * Git Bash
/// * Console host for double clicking in Windows Explorer:
///   * Windows Console Host
///   * Windows Terminal
///
/// ## Alternatives
/// We have tried `#![windows_subsystem = "windows"]` plus `AttachConsole`/`AllocConsole`,
/// but that messes up the console output in Command Prompt and PowerShell
/// (a new prompt line is shown, and then the output bleeds into that line).
///
/// We have tried relaunching the program with a special environment variable,
/// but that eventually raised a false positive from Windows Defender (`Win32/Wacapew.C!ml`).
///
/// We may eventually want to try using a manifest to set `<consoleAllocationPolicy>`,
/// but that is not yet widely available:
/// https://github.com/microsoft/terminal/blob/5383cb3a1bb8095e214f7d4da085ea4646db8868/doc/specs/%237335%20-%20Console%20Allocation%20Policy.md
///
/// ## Considerations
/// The current approach is to let the console appear and then immediately `FreeConsole`.
/// Previously, Windows Terminal wouldn't remove the console in that case,
/// but that has been fixed: https://github.com/microsoft/terminal/issues/16174
///
/// There was also an issue where asynchronous Rclone commands would fail to spawn
/// ("The request is not supported (os error 50)"),
/// but that has been solved by resetting the standard device handles:
/// https://github.com/rust-lang/rust/issues/113277
///
/// Watch out for non-obvious code paths that may defeat detachment.
/// flexi_logger's `colors` feature would cause the console to stick around
/// if logging was enabled before detaching.
#[cfg(target_os = "windows")]
unsafe fn detach_console() {
    use windows::Win32::{
        Foundation::HANDLE,
        System::Console::{FreeConsole, SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE},
    };

    fn tell(msg: &str) {
        eprintln!("{}", msg);
        log::error!("{}", msg);
    }

    if FreeConsole().is_err() {
        tell("Unable to detach the console");
        std::process::exit(1);
    }
    if SetStdHandle(STD_INPUT_HANDLE, HANDLE::default()).is_err() {
        tell("Unable to reset stdin handle");
        std::process::exit(1);
    }
    if SetStdHandle(STD_OUTPUT_HANDLE, HANDLE::default()).is_err() {
        tell("Unable to reset stdout handle");
        std::process::exit(1);
    }
    if SetStdHandle(STD_ERROR_HANDLE, HANDLE::default()).is_err() {
        tell("Unable to reset stderr handle");
        std::process::exit(1);
    }
}

fn main() {
    let mut failed = false;

    let logger = prepare_logging();
    #[allow(clippy::useless_asref)]
    prepare_panic_hook(logger.as_ref().map(|x| x.clone()).ok());
    let flush_logger = || {
        if let Ok(logger) = &logger {
            logger.flush();
        }
    };

    log::debug!("Version: {}", *VERSION);
    log::debug!("Invocation: {:?}", std::env::args());

    let args = match cli::parse() {
        Ok(x) => x,
        Err(e) => {
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {}
                _ => {
                    log::error!("CLI failed to parse: {e}");
                }
            }
            flush_logger();
            e.exit()
        }
    };

    if let Some(config_dir) = args.config.as_deref() {
        *CONFIG_DIR.lock().unwrap() = Some(config_dir.to_path_buf());
    }

    match args.sub {
        None => {
            // Do any extra CLI parsing before we detach the console.
            let mut sources = cli::parse_sources(args.sources);
            sources.extend(args.glob.into_iter().map(media::Source::new_glob));

            #[cfg(target_os = "windows")]
            if std::env::var(crate::prelude::ENV_DEBUG).is_err() {
                unsafe {
                    detach_console();
                }
            }

            let flags = Flags { sources };
            gui::run(flags);
        }
        Some(sub) => {
            if let Err(e) = cli::run(sub) {
                failed = true;
                eprintln!("{}", lang::handle_error(&e));
            }
        }
    };

    flush_logger();

    if failed {
        std::process::exit(1);
    }
}
