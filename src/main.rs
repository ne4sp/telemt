//! telemt — Telegram MTProto Proxy

mod api;
mod cli;
mod config;
mod crypto;
#[cfg(unix)]
mod daemon;
mod error;
mod ip_tracker;
#[cfg(test)]
#[path = "tests/ip_tracker_hotpath_adversarial_tests.rs"]
mod ip_tracker_hotpath_adversarial_tests;
#[cfg(test)]
#[path = "tests/ip_tracker_encapsulation_adversarial_tests.rs"]
mod ip_tracker_encapsulation_adversarial_tests;
#[cfg(test)]
#[path = "tests/ip_tracker_regression_tests.rs"]
mod ip_tracker_regression_tests;
mod maestro;
mod metrics;
mod network;
mod protocol;
mod proxy;
mod startup;
mod stats;
mod stream;
mod tls_front;
mod transport;
mod util;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // On Unix, handle daemonization before starting tokio runtime
    #[cfg(unix)]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let daemon_opts = cli::parse_daemon_args(&args);

        // Daemonize if requested (must happen before tokio runtime starts)
        if daemon_opts.should_daemonize() {
            match daemon::daemonize(daemon_opts.working_dir.as_deref()) {
                Ok(daemon::DaemonizeResult::Parent) => {
                    // Parent process exits successfully
                    std::process::exit(0);
                }
                Ok(daemon::DaemonizeResult::Child) => {
                    // Continue as daemon child
                }
                Err(e) => {
                    eprintln!("[telemt] Daemonization failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // Now start tokio runtime and run the server
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(maestro::run_with_daemon(daemon_opts))
    }

    #[cfg(not(unix))]
    {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(maestro::run())
    }
}
