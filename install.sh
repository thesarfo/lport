#!/usr/bin/env bash
# install or remove lport for the current user.
#   ./install.sh              install (uses a prebuilt ./lport if present, otherwise builds)
#   ./install.sh uninstall    remove it
set -euo pipefail

cd "$(dirname "$0")"

prefix="${PREFIX:-$HOME/.local}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
config="${XDG_CONFIG_HOME:-$HOME/.config}"

bin="$prefix/bin/lport"
menu="$data/applications/lport.desktop"
autostart="$config/autostart/lport.desktop"

case "${1:-install}" in
install)
    if [ -x ./lport ]; then
        src=./lport
    else
        cargo build --release
        src=target/release/lport
    fi
    install -Dm755 "$src" "$bin"
    mkdir -p "$(dirname "$menu")" "$(dirname "$autostart")"
    sed "s|@BIN@|$bin|" assets/lport.desktop >"$menu"
    sed "s|@BIN@|$bin|" assets/lport-autostart.desktop >"$autostart"
    command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" || true
    echo "installed $bin"

    case ":$PATH:" in
    *":$prefix/bin:"*) ;;
    *) echo "add $prefix/bin to your PATH to run: lport" ;;
    esac
    echo "start it now with: $bin &"
    ;;
uninstall)
    pkill -x lport 2>/dev/null || true
    rm -f "$bin" "$menu" "$autostart"
    echo "removed lport"
    ;;
*)
    echo "usage: $0 [install|uninstall]" >&2
    exit 2
    ;;
esac
