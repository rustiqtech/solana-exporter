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
to set up a default configuration file, and the directory where the exporter's persistent database will live.

By default, the `generate` command will place a config file inside `~/.solana-exporter`.