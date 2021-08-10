# Docker

`solana-exporter` is also available as a container.

```shell
docker run -d -v /path/to/your/config.toml:/etc/solana-exporter/config.toml -v solana-exporter-data:/exporter solana-exporter
```
TODO: What is the full name of the container on Dockerhub?