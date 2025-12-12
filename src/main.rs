#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        let command_trim= command.trim();
        if command_trim == "exit" {
            break;
        }else if command_trim.starts_with("echo") {
            println!("{}", command_trim.strip_prefix("echo ").unwrap());
        }else{
            println!("{}: command not found", command_trim);
        }
    }
}
