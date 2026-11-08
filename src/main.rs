mod procnet;
mod tray;

use std::{env, process, thread, time::Duration};

use ksni::blocking::TrayMethods;

const USAGE: &str = "\
lport: see and stop what is listening on your ports

usage:
  lport             run the tray icon
  lport list        print listening ports
  lport kill PORT   stop the process listening on PORT (SIGTERM)
";

const POLL: Duration = Duration::from_secs(2);

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => run_tray(),
        ["list"] => list(),
        ["kill", port] => kill(port),
        ["-h" | "--help"] => print!("{USAGE}"),
        ["-V" | "--version"] => println!("lport {}", env!("CARGO_PKG_VERSION")),
        _ => {
            eprint!("{USAGE}");
            process::exit(2);
        }
    }
}

fn run_tray() {
    let handle = match (tray::Tray { listeners: procnet::listeners() }).spawn() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("lport: no tray ({e}). On GNOME, enable the AppIndicator extension.");
            process::exit(1);
        }
    };
    loop {
        thread::sleep(POLL);
        let now = procnet::listeners();
        let changed = handle.update(|t| {
            let changed = t.listeners != now;
            if changed {
                t.listeners = now;
            }
            changed
        });
        if changed.is_none() {
            return; // tray service shut down
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
