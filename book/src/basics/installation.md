# Installation

To install the latest version of `solana-exporter`, run:
```
cargo install solana-exporter
```

Note that the libraries `libssl` and `libudev` should be installed on the target machine as well. Refer to your
distro's package manager and documentation for guidance on how to install them.

After installation, run
```
solana-exporter generate
```
to set up a default configuration file. By default, the `generate` command will place a config file inside
`~/.solana-exporter`; this directory will be automatically created, unless the `-o` flag is present to override
the output location.

## Running as a service

Run this as a systemd service by a non-root user with a script like this one:
```
[Unit]
Description=Solana Exporter
After=solana.service
Requires=solana.service

[Service]
User=solana
Restart=always
RestartSec=20
ExecStart=/home/solana/.cargo/bin/solana-exporter

[Install]
WantedBy=multi-user.target
```

# Running as a Docker container

`solana-exporter` is also available as a container which can be pulled by

```shell
docker pull rustiq/solana-exporter:0.4.0
```

Then run it with
```shell
docker run -d \
-v /path/to/your/config.toml:/etc/solana-exporter/config.toml \
-v solana-exporter-data:/exporter solana-exporter
```

We recommend that the `config.toml` file be bind-mounted to the container, so you have easy access to it on the host
machine. However, the persistent database should be stored in a named volume.
