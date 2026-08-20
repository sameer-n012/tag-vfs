use std::env;
use tag_vfs::app::command_line_app::CommandLineApp;
use tag_vfs::app::run_configuration::RunConfiguration;

fn main() {
    let mut config = RunConfiguration::new(env::args());
    // Config files are optional; missing files are silently ignored.
    let _ = config.parse_default_config_file();
    let _ = config.parse_user_config_file();
    if let Err(e) = config.parse_command_line_args() {
        eprintln!("Error parsing command line arguments: {}", e);
        std::process::exit(1);
    }

    let mut app = CommandLineApp::new(config);
    app.run();
}
