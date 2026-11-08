//! Who is listening on which TCP port, read straight from /proc. No root needed,
//! but we can only see the owner of sockets that belong to our own user.

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use rustix::process::{Pid, Signal, kill_process};

#[derive(Clone, Debug, PartialEq)]
pub struct Listener {
    pub port: u16,
    pub addr: IpAddr,
    /// None when the socket belongs to another user (or root).
    pub owner: Option<Proc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proc {
    pub pid: i32,
    pub name: String,
    /// Last part of the process's working directory, e.g. "somu-fe".
    pub project: Option<String>,
}

impl Listener {
    pub fn label(&self) -> String {
        match &self.owner {
            Some(p) => match &p.project {
                Some(dir) => format!(":{}  {} · {}", self.port, p.name, dir),
                None => format!(":{}  {}", self.port, p.name),
            },
            None => format!(":{}", self.port),
        }
    }
}

/// Every listening TCP port, one row per port, sorted.
pub fn listeners() -> Vec<Listener> {
    let mut socks = Vec::new();
    for path in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(text) = fs::read_to_string(path) {
            socks.extend(text.lines().skip(1).filter_map(parse_row));
        }
    }
    let wanted: HashSet<u64> = socks.iter().map(|s| s.2).collect();
    let owners = socket_owners(&wanted);

    let mut out: Vec<Listener> = socks
        .into_iter()
        .map(|(addr, port, inode)| Listener { port, addr, owner: owners.get(&inode).cloned() })
        .collect();
    // a server often listens on both 0.0.0.0 and [::]; show it once, preferring a known owner
    out.sort_by_key(|l| (l.port, l.owner.is_none(), l.addr.is_ipv6()));
    out.dedup_by_key(|l| l.port);
    out
}

/// Send SIGTERM to whoever owns `port` right now. Looked up fresh so a stale
/// menu can never kill a process that has since reused the pid.
pub fn kill_port(port: u16) -> io::Result<i32> {
    let owner = listeners()
        .into_iter()
        .find(|l| l.port == port)
        .and_then(|l| l.owner)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("nothing of yours on :{port}")))?;
    let pid = Pid::from_raw(owner.pid).ok_or_else(|| io::Error::other("bad pid"))?;
    kill_process(pid, Signal::TERM)?;
    Ok(owner.pid)
}

const LISTEN: &str = "0A";

/// A row of /proc/net/tcp{,6} as (address, port, socket inode), if it is listening.
fn parse_row(line: &str) -> Option<(IpAddr, u16, u64)> {
    // sl local_address rem_address st tx:rx tr:when retrnsmt uid timeout inode ...
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 10 || f[3] != LISTEN {
        return None;
    }
    let (ip, port) = f[1].split_once(':')?;
    Some((parse_ip(ip)?, u16::from_str_radix(port, 16).ok()?, f[9].parse().ok()?))
}

/// The kernel prints each 32-bit word of the address in host byte order.
fn parse_ip(hex: &str) -> Option<IpAddr> {
    let mut bytes = Vec::with_capacity(16);
    for i in (0..hex.len()).step_by(8) {
        let word = u32::from_str_radix(hex.get(i..i + 8)?, 16).ok()?;
        bytes.extend(word.to_ne_bytes());
    }
    match bytes.len() {
        4 => Some(Ipv4Addr::from(<[u8; 4]>::try_from(bytes).ok()?).into()),
        16 => Some(Ipv6Addr::from(<[u8; 16]>::try_from(bytes).ok()?).into()),
        _ => None,
    }
}

/// Map socket inodes to the process holding them. Other users' /proc/<pid>/fd
/// is unreadable, so their sockets simply don't show up here.
fn socket_owners(wanted: &HashSet<u64>) -> HashMap<u64, Proc> {
    let mut map = HashMap::new();
    let Ok(dir) = fs::read_dir("/proc") else { return map };
    for entry in dir.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else {
            continue;
        };
        let Ok(fds) = fs::read_dir(entry.path().join("fd")) else { continue };
        let mut proc = None;
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else { continue };
            let inode = target
                .to_str()
                .and_then(|t| t.strip_prefix("socket:["))
                .and_then(|t| t.strip_suffix(']'))
                .and_then(|t| t.parse::<u64>().ok());
            if let Some(inode) = inode.filter(|i| wanted.contains(i)) {
                let p = proc.get_or_insert_with(|| describe(pid)).clone();
                map.insert(inode, p);
            }
        }
    }
    map
}

fn describe(pid: i32) -> Proc {
    let base = format!("/proc/{pid}");
    let name = fs::read_to_string(format!("{base}/comm")).unwrap_or_default().trim().to_string();
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let project = fs::read_link(format!("{base}/cwd"))
        .ok()
        .filter(|cwd| cwd.parent().is_some() && Some(cwd) != home.as_ref())
        .and_then(|cwd| cwd.file_name().map(|n| n.to_string_lossy().into_owned()));
    Proc { pid, name, project }
}

#[cfg(all(test, target_endian = "little"))]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_listener() {
        let row = "   0: 0100007F:1435 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 98765 1 0000000000000000 100 0 0 10 0";
        assert_eq!(parse_row(row), Some((Ipv4Addr::LOCALHOST.into(), 5173, 98765)));
    }

    #[test]
    fn parses_ipv6_any() {
        let row = "   1: 00000000000000000000000000000000:1F90 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4242 1 0000000000000000 100 0 0 10 0";
        assert_eq!(parse_row(row), Some((Ipv6Addr::UNSPECIFIED.into(), 8080, 4242)));
    }

    #[test]
    fn skips_established() {
        let row = "   2: 0100007F:1435 0100007F:C350 01 00000000:00000000 00:00000000 00000000  1000        0 111 1 0000000000000000 20 4 30 10 -1";
        assert_eq!(parse_row(row), None);
    }

    #[test]
    fn finds_our_own_listener() {
        let sock = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = sock.local_addr().unwrap().port();
        let me = listeners().into_iter().find(|l| l.port == port).expect("listener visible");
        assert_eq!(me.owner.unwrap().pid, std::process::id() as i32);
    }
}
