mod procnet;

use std::{env, process};

const USAGE: &str = "\
lport: see and stop what is listening on your ports

usage:
  lport list        print listening ports
  lport kill PORT   stop the process listening on PORT (SIGTERM)
";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["list"] => list(),
        ["kill", port] => kill(port),
        ["-h" | "--help"] => print!("{USAGE}"),
        ["-V" | "--version"] => println!("lport {}", env!("CARGO_PKG_VERSION")),
        _ => {
            eprint!("{USAGE}");
            process::exit(2);
        }
    }
}

fn list() {
    for l in procnet::listeners() {
        match &l.owner {
            Some(p) => println!("{:<7}{:<8}{:<16}{}", l.port, p.pid, p.name, p.project.as_deref().unwrap_or("")),
            None => println!("{:<7}{:<8}(other user)", l.port, "-"),
        }
    }
}

fn kill(port: &str) {
    let Ok(port) = port.parse::<u16>() else {
        eprintln!("lport: {port} is not a port");
        process::exit(2);
    };
    match procnet::kill_port(port) {
        Ok(pid) => println!("sent SIGTERM to pid {pid} on :{port}"),
        Err(e) => {
            eprintln!("lport: {e}");
            process::exit(1);
        }
    }
}
