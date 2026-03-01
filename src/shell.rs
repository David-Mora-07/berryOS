use alloc::string::String;
use crate::{print, println};

pub struct Shell {
    input: String,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            input: String::new(),
        }
    }

    pub fn handle_key(&mut self, key: char) {
        match key {
            '\n' => {
                println!();
                self.execute();
                print!("> ");
            }
            '\x08' => {
                self.input.pop();
                print!("{}", key);
            }
            c => {
                self.input.push(c);
                print!("{}", c);
            }
        }
    }

    fn execute(&mut self) {
        match self.input.as_str() {
            "help" => println!("Comandos: help, clear, echo"),
            "clear" => {
                for _ in 0..50 {
                    println!();
                }
            }
            cmd if cmd.starts_with("echo ") => {
                println!("{}", &cmd[5..]);
            }
            _ => println!("Comando no encontrado: {}", self.input),
        }
        self.input.clear();
    }
}