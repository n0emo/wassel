@_default:
    just --list

prepare:
    cargo clippy --fix --allow-dirty -- -D warnings
    cargo test
    cargo fmt

wit:
    wkg wit fetch -t wit

rust-plugin:
    mkdir -p plugins/rust-plugin
    cargo build \
        --manifest-path ./examples/rust-plugin/Cargo.toml \
        --target-dir ./target/ \
        --target wasm32-wasip2
    cp  ./target/wasm32-wasip2/debug/hello_plugin.wasm \
        ./plugins/rust-plugin/plugin.wasm
    cp  ./examples/rust-plugin/plugin.toml \
        ./plugins/rust-plugin/plugin.toml
