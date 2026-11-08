# lport

See what is listening on your ports, and stop it, from the tray on Linux.

Each port shows the process and the folder it was started from, like `:5173 node · somu-fe`.

## Install

```sh
git clone git@github.com:thesarfo/lport.git
cd lport
./install.sh
```

Or unpack a release tarball and run `./install.sh` inside it.

This installs the `lport` command, an app menu entry, and autostart. Remove everything with `./install.sh uninstall`.

## Use

| Command | What it does |
| --- | --- |
| `lport` | Run the tray icon |
| `lport list` | Print listening ports |
| `lport kill PORT` | Stop whatever is listening on PORT |

In the tray menu, each port has Open in browser and Kill.

## Notes

- Reads `/proc`, so no root. Ports owned by root or other users, including Docker, are listed under Other users and can't be killed.
- Kill sends `SIGTERM`. The port is looked up again first, so a stale menu never hits the wrong process.
- On GNOME you need the AppIndicator extension for the tray.

## Build

```sh
cargo build --release
```
