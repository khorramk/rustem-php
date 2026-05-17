# Linux Build

This project can build a Linux `rustenphp` binary in Docker.

```bash
docker build -f Dockerfile.linux --target builder -t rustenphp-linux-builder .
docker create --name rustenphp-linux-out rustenphp-linux-builder
docker cp rustenphp-linux-out:/out ./dist-linux
docker rm rustenphp-linux-out
```

The binary is written to:

```text
dist-linux/rustenphp
```

It dynamically links to Linux PHP embed libraries. On a Linux host, install the matching runtime:

```bash
sudo apt-get install libphp8.2-embed php8.2-cli
```

You can also run the packaged test image directly:

```bash
docker build -f Dockerfile.linux -t rustenphp-linux .
docker run --rm -p 8787:8787 rustenphp-linux
```

To run it with resource limits:

```bash
docker run --rm -p 8787:8787 --memory=1g --cpus=0.25 rustenphp-linux
```

`--cpus=0.25` means 25% of one CPU core. If you truly want 0.25% of one CPU core, use `--cpus=0.0025`, but that is likely too low for a usable PHP/Laravel runtime.

The same limits are available through Docker Compose:

```bash
docker compose up --build
```

Then open:

```text
http://127.0.0.1:8787
```
