# Introduction

`solana-exporter` is an advanced, modular monitoring solution for
[Solana](https://github.com/solana-labs/solana) validator nodes. The executable part is implemented
in Rust and was initially based on the [Golang
original](https://github.com/certusone/solana_exporter) by CertusOne but now provides additional
functionality. It comprises

- a Prometheus exporter executable written in Rust,

- a Docker container suitable for `docker-compose`,

- sample Grafana dashboards.

Typical uses include

- monitoring Solana validator nodes by the nodes' operator,

- managing a stake pool,

- monitoring the cluster health.

This guide explains how to set the monitoring stack up and how to stay on top of things using
Grafana alerts.
