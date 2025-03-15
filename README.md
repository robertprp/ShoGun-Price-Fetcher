## Shogun Test Price fetcher

For using docker, you must create a file named `config.toml` with configuration as it comes from `config.example.toml`

### Cargo & Rust
```bash
cargo build --workspace -r
# You may use --config and --log-level for extra config. Default values are `config.toml` and `INFO` respectively.
./target/release/grafana-shogun
```

### Docker (best approach)
To start up all services, use:
```bash
docker compose -f docker/docker-compose.yaml up --build -d
```
