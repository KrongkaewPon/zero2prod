### Test and run

`cargo watch -x check -x test -x run`
`TEST_LOG=true cargo test`

### Create database

`SKIP_DOCKER=true ./scripts/init_db.sh`

### Run with log level trace

`RUST_LOG=trace cargo run`

### Docker

`docker build --tag zero2prod --file Dockerfile .`
`docker images zero2prod `
`docker run -p 8000:8000 zero2prod`
