<p align="center">
  <img src="../mooplas_game/assets/ignore/logo.png" width="400" height="100" alt="Randolf"/>
</p>

This crate contains a standalone Mooplas signalling server for the Matchbox WASM backend. It supports:

1. Plain `ws://` for local development
2. TLS-terminated `wss://` when you provide PEM certificate and key files
3. A simple `/health` endpoint for monitoring

## Running locally

Run with it:

```shell
cargo run -p mooplas_signalling_server -- --port 3536
```

Or run with TLS / `wss://` with:

```bash
cargo run -p mooplas_signalling_server -- --port 443 --tls-cert <PATH> --tls-key <PATH>
```

The binary accepts:

- `--port` — defaults to `3536`
- `--tls-cert <PATH>` — PEM certificate chain
- `--tls-key <PATH>` — PEM private key

`--tls-cert` and `--tls-key` must be supplied together. If both are omitted, the server stays in plain `ws://` mode for
local development.

When the page hosting the game is served over HTTPS, browser clients must connect to this server over `wss://`.

To check that it works, do a health check with:

```bash
curl http://127.0.0.1:3536/health -v
```

## Container image

### Build

From the root of the repository:

```bash
docker build -f mooplas_signalling_server/Dockerfile -t mooplas-signalling-server .
```

### Run

Plain `ws://` (e.g. local development):

```shell
docker run --rm -p 3536:3536 mooplas-signalling-server
```

With TLS / `wss://`:

```bash
docker run --rm \
  -p 443:443 \
  -v /path/to/certs:/certs:ro \
  mooplas-signalling-server \
  --port 443 \
  --tls-cert /certs/fullchain.pem \
  --tls-key /certs/privkey.pem
```

### Health check

```bash
curl http://127.0.0.1:3536/health -v
```
