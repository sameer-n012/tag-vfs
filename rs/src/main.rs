mod app;
mod archive;
mod data;
mod exceptions;
mod loader;
mod util;

use app::command_line_app::CommandLineApp;
use app::run_configuration::RunConfiguration;
use std::env;

fn main() {
    let mut config = RunConfiguration::new(env::args());
    if let Err(e) = config.parse_default_config_file() {
        eprintln!("Error parsing default config file: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = config.parse_user_config_file() {
        eprintln!("Error parsing user config file: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = config.parse_command_line_args() {
        eprintln!("Error parsing command line arguments: {}", e);
        std::process::exit(1);
    }

    if config.get_config_bool("gui") {
        println!("GUI not yet supported");
        std::process::exit(0);
    } else {
        let mut app = CommandLineApp::new(config);
        app.run();
    }
}
