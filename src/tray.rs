use std::process::{Command, Stdio};

use ksni::{
    MenuItem,
    menu::{StandardItem, SubMenu},
};

use crate::procnet::{self, Listener};

pub struct Tray {
    pub listeners: Vec<Listener>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "lport".into()
    }

    fn title(&self) -> String {
        "lport".into()
    }

    fn icon_name(&self) -> String {
        "network-server-symbolic".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let mine = self.listeners.iter().filter(|l| l.owner.is_some()).count();
        ksni::ToolTip { title: format!("{mine} of your ports listening"), ..Default::default() }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let (mine, other): (Vec<_>, Vec<_>) = self.listeners.iter().partition(|l| l.owner.is_some());
        let mut menu: Vec<MenuItem<Self>> = mine.iter().map(|l| port_menu(l)).collect();
        if menu.is_empty() {
            menu.push(disabled("Nothing of yours is listening"));
        }
        if !other.is_empty() {
            menu.push(MenuItem::Separator);
            menu.push(
                SubMenu {
                    label: format!("Other users ({})", other.len()),
                    submenu: other.iter().map(|l| disabled(&l.label())).collect(),
                    ..Default::default()
                }
                .into(),
            );
        }
        menu.push(MenuItem::Separator);
        menu.push(item("Quit", |_| std::process::exit(0)));
        menu
    }
}

fn port_menu(l: &Listener) -> MenuItem<Tray> {
    let port = l.port;
    let pid = l.owner.as_ref().map_or(0, |p| p.pid);
    SubMenu {
        label: l.label(),
        submenu: vec![
            item("Open in browser", move |_| open(port)),
            item(&format!("Kill (pid {pid})"), move |_| {
                if let Err(e) = procnet::kill_port(port) {
                    eprintln!("lport: kill :{port}: {e}");
                }
            }),
        ],
        ..Default::default()
    }
    .into()
}

fn open(port: u16) {
    let url = format!("http://localhost:{port}");
    let spawned = Command::new("xdg-open").arg(&url).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    if let Err(e) = spawned {
        eprintln!("lport: open {url}: {e}");
    }
}

fn item(label: &str, f: impl Fn(&mut Tray) + Send + 'static) -> MenuItem<Tray> {
    StandardItem { label: label.into(), activate: Box::new(f), ..Default::default() }.into()
}

fn disabled(label: &str) -> MenuItem<Tray> {
    StandardItem { label: label.into(), enabled: false, ..Default::default() }.into()
}
