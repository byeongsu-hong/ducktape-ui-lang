#!/bin/sh
set -eu

if [ "${UI_LANG_A11Y_SMOKE_SESSION:-}" != "1" ]; then
  exec dbus-run-session -- env UI_LANG_A11Y_SMOKE_SESSION=1 "$0" "$@"
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(dirname -- "$script_dir")
cd "$repo_root"

address_result=$(gdbus call --session \
  --dest org.a11y.Bus \
  --object-path /org/a11y/bus \
  --method org.a11y.Bus.GetAddress)
address=${address_result#\(\'}
address=${address%\',\)}
export AT_SPI_BUS_ADDRESS=$address

/usr/libexec/at-spi2-registryd --dbus-name org.a11y.atspi.Registry >/dev/null 2>&1 &
registry_pid=$!
cleanup() {
  kill "$registry_pid" 2>/dev/null || true
  wait "$registry_pid" 2>/dev/null || true
}
trap cleanup EXIT HUP INT TERM

cargo test -p ui-lang-runtime linux_native_atspi_exports_tree_and_routes_action \
  -- --ignored --nocapture --test-threads=1 "$@"
