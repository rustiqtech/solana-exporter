# Docker

`solana-exporter` is also available as a container.

```shell
docker run -d \
-v /path/to/your/config.toml:/etc/solana-exporter/config.toml \
-v solana-exporter-data:/exporter solana-exporter
```
TODO: What is the full name of the container on Dockerhub?

We recommend that the `config.toml` file be bind-mounted to the container, so you have easy access to it on the host
machine. However, the persistent database should be stored in a named volume.