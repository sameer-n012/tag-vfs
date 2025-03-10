use crate::app::app::App;
use crate::app::run_configuration::RunConfiguration;
use std::io::{self, Write};

pub struct CommandLineApp {
    app: App,
}

impl CommandLineApp {
    pub fn new(config: RunConfiguration) -> Self {
        CommandLineApp {
            app: App::new(config),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut input = String::new();

        loop {
            print!("{} ", self.app.config.get_config_string("cliPrefix"));
            stdout.flush().unwrap();
            stdin.read_line(&mut input).unwrap();
            let command = input.trim();
            if self.eval_command(command) {
                break;
            }
            input.clear();
        }
    }

    fn eval_command(&mut self, command: &str) -> bool {
        match command {
            "quit" => self.cli_quit(),
            "help" => self.cli_help(),
            // "open" => cli_open(cmd);
            // "apply" => cli_apply(cmd);
            // "expand" => cli_expand(cmd);
            // "reduce" => cli_reduce(cmd);
            // "import" => cli-import(cmd);
            // "remove" => cli_remove(cmd);
            // "destroy" => cli_destroy(cmd);
            // "merge" => cli_merge(cmd);
            // "scrape" => cli_scrape(cmd);
            // "config" => cli_config(cmd);
            // "flush" => cli_flush(cmd);
            // "ls" => cli_list(cmd);
            // "sz" => cli_size(cmd);
            // "tag" => cli_tag(cmd);
            _ => {
                println!("Unknown command: {}. Type 'help' for more.", command);
                false
            }
        }
    }

    fn cli_quit(&self) -> bool {
        println!("Are you sure you want to quit? (y/n): ");
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().eq_ignore_ascii_case("y") {
            self.app.clean();
            true
        } else {
            false
        }
    }

    fn cli_help(&self) -> bool {
        println!("Help menu:");
        println!("Available commands: help, quit");
        false
    }
}
