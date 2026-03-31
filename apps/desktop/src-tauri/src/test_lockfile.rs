use std::fs;

fn main() {
    let path = "C:\\Riot Games\\League of Legends\\lockfile";
    match fs::read_to_string(path) {
        Ok(content) => println!("Success: {}", content),
        Err(e) => println!("Error reading lockfile: {}", e),
    }
}
