set shell := ["bash", "-euo", "pipefail", "-c"]

prefix := env_var_or_default("PREFIX", env_var("HOME") / ".local")
bindir := prefix / "bin"
plugindir := prefix / "share" / "foothold" / "plugins"

qmldir := if prefix == "/usr" { "/etc/xdg/quickshell/foothold" } else { env_var("HOME") / ".config/quickshell/foothold" }

_default:
    @just --list

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

build:
    cargo build --workspace --release

dev:
    cargo build --workspace
    install -Dm755 target/debug/foothold {{bindir}}/foothold
    install -Dm755 target/debug/foothold-config {{bindir}}/foothold-config
    just _plugins target/debug
    mkdir -p $(dirname {{qmldir}})
    rm -rf {{qmldir}}
    ln -sfn {{justfile_directory()}}/qml {{qmldir}}
    @echo "installed to {{bindir}}, qml symlinked to the repo"

install: build
    install -Dm755 target/release/foothold {{bindir}}/foothold
    install -Dm755 target/release/foothold-config {{bindir}}/foothold-config
    just _plugins target/release
    rm -rf {{qmldir}}
    mkdir -p {{qmldir}}
    install -Dm644 qml/*.qml {{qmldir}}/
    @echo "installed to {{prefix}}"

_plugins profile:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    for manifest in plugins/*/plugin.toml; do
        name=$(basename "$(dirname "$manifest")")
        bin=$(sed -n 's/^bin *= *"\(.*\)"/\1/p' "$manifest")
        install -Dm644 "$mainfest" "{{plugindir}}/$name/plugin.toml"
        install -Dm755 "{{profile}}/$bin" "{{plugindir}}/$name/$bin"
        echo "  plugin $name"
    done

uninstall:
    rm -f {{bindir}}/foothold {{bindir}}/foothold-config
    rm -rf {{prefix}}/share/foothold
    rm -rf {{qmldir}}
    @echo "removed from {{prefix}}"

# Restart the running shell against the installed copy
restart:
    -pkill -f 'qs -c foothold'
    setsid qs -c foothold > /dev/null 2>&1 &
    @echo "restarted"

log:
    journalctl --user -f -o cat | grep -i 'qs\['
