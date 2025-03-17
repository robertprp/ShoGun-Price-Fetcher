## Shogun Test Price fetcher

For using docker, you must create a file named `config.toml` with configuration as it comes from `config.example.toml`

### Cargo & Rust
```bash
cargo build --workspace -r
# You may use --config and --log-level for extra config. Default values are `config.toml` and `INFO` respectively.
./target/release/grafana-shogun
```

### Docker (best approach)
Update `config.toml` with the following docker specific configuration:
```toml
[tasks]

[tasks.fetcher] 
interval = 10 # seconds

[environment]
name = "local-docker"
otlp_grpc_endpoint = "http://otlp-collector:4317"
otlp_http_endpoint = "http://otlp-collector:4318"
```

To start up all services, use:
```bash
docker compose -f docker/docker-compose.yaml up --build -d
```

To stop all services, use:
```bash
docker compose -f docker/docker-compose.yaml down
```

To start up only one service, use:
```bash
docker compose -f docker/docker-compose.yaml up --build -d grafana
```
## Grafana

Grafana is available at `http://localhost:33000`

Default credentials are:
- Username: `admin`
- Password: `admin`

Logs will be available at Loki.
Prometheus will have some data.
Tempo should contain the executed price fetching action traces.