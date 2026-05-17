# RustenPHP Embed

RustenPHP is an experimental Rust HTTP server that embeds PHP in-process and executes a PHP front controller such as Laravel's `public/index.php`.

This is a prototype, not a production runtime yet.

## Boot A Fresh Laravel App

Create a Laravel app beside this repository:

```bash
cd C:\Users\korom\Documents\Codex\2026-05-16\i-want-you-to-create-a
composer create-project laravel/laravel fresh-laravel
cd fresh-laravel
php artisan key:generate
```

For a quick local test, use SQLite:

```bash
type nul > database\database.sqlite
```

Set these values in `fresh-laravel\.env`:

```env
APP_URL=http://127.0.0.1:8787
DB_CONNECTION=sqlite
```

Then build the Linux RustenPHP image from this repo:

```bash
cd C:\Users\korom\Documents\Codex\2026-05-16\i-want-you-to-create-a\rustenphp-embed
docker build -f Dockerfile.linux -t rustenphp-linux .
```

Run the fresh Laravel app through the embedded PHP runtime:

```bash
docker run --rm ^
  -p 8787:8787 ^
  --memory=1g ^
  --cpus=0.25 ^
  -e RUSTENPHP_HOST=0.0.0.0 ^
  -e RUSTENPHP_PORT=8787 ^
  -e RUSTENPHP_LARAVEL_ROOT=/app ^
  -v "C:\Users\korom\Documents\Codex\2026-05-16\i-want-you-to-create-a\fresh-laravel:/app" ^
  rustenphp-linux
```

Open:

```text
http://127.0.0.1:8787
```

## Linux Host Run

The Linux binary is exported to:

```text
dist-linux/rustenphp
```

It dynamically links to Linux PHP embed libraries. On a Debian/Ubuntu server, install the matching runtime dependencies:

```bash
sudo apt-get update
sudo apt-get install -y \
  libphp8.2-embed \
  php8.2-cli \
  php8.2-curl \
  php8.2-mbstring \
  php8.2-pgsql \
  php8.2-sqlite3 \
  php8.2-xml \
  php8.2-zip
```

Then run from your Laravel app directory:

```bash
export RUSTENPHP_HOST=0.0.0.0
export RUSTENPHP_PORT=8787
export RUSTENPHP_LARAVEL_ROOT=/path/to/your/laravel-app
./dist-linux/rustenphp serve
```

## Current Limits

- Handles `GET` and `HEAD` only.
- Captures response body, but does not yet forward PHP/Laravel headers correctly.
- Boots PHP per dynamic request.
- Laravel must already have `vendor/` installed.
- This is not a safe production runtime yet.
