cargo watch -x check -x test -x run

SKIP_DOCKER=true ./scripts/init_db.sh

RUST_LOG=trace cargo run
