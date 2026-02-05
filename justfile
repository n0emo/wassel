[parallel]
wit-fetch: wit-fetch-root wit-fetch-python-plugin wit-fetch-rust-plugin

@wit-fetch-root:
    echo "Fetching WIT dependencies for Wassel"
    wkg wit fetch -t wit

[working-directory("examples/python-plugin")]
@wit-fetch-python-plugin:
    echo "Fetching WIT dependencies for Python plugin example"
    wkg wit fetch -t wit

[working-directory("examples/rust-plugin")]
@wit-fetch-rust-plugin:
    echo "Fetching WIT dependencies for Rust plugin example"
    wkg wit fetch -t wit
