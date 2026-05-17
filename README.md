# RustenPHP-Embed 🚀

A hyper-optimized, ultra-lean application server and runtime for Laravel, built entirely in Rust. By embedding the C-based PHP Zend Engine directly into a native Rust binary via Foreign Function Interface (FFI), RustenPHP completely eliminates the need for traditional web servers like Nginx, Apache, or PHP-FPM pools.

Instead of booting and tearing down the framework on every request, RustenPHP boots Laravel **exactly once** into a persistent background worker thread, serving concurrent requests in double-digit milliseconds while utilizing a fraction of the RAM of a traditional stack.

## ✨ Key Features

* **Zero-Middleware Architecture:** No Nginx. No FastCGI network serialization protocols. Rust passes HTTP payloads directly to PHP using native memory pointers.
* **Persistent Framework Workers:** Takes inspiration from Laravel Octane. Your application kernel stays "warm" in memory, dropping execution latency from hundreds of milliseconds to near-instant speeds.
* **Microscopic Resource Footprint:** Idles as low as ~23MB–40MB of RAM, scaling gracefully under load. Perfect for low-resource environments (like 128MB RAM embedded systems or tiny 0.25 vCPU cloud instances).
* **Asynchronous Multi-Threaded Networking:** Uses native Rust `std::thread` workers to ingest TCP streams concurrently, routing tasks down a secure channel to the isolated PHP engine.

---

## 📋 System Requirements

Before compiling or running the server, ensure your environment matches the following:

### Windows
* **PHP:** 8.2+ compiled with Thread Safety (ZTS is recommended but NTS works via our single-worker pipeline).
* **Visual Studio Build Tools:** C++ workload installed (required to link against `php8embed.lib`).
* **Laravel Framework:** 10.x / 11.x app structure.

### Linux / Docker
* `build-essential`, `libssl-dev`, and standard PHP embed headers installed.

---

## 🛠️ Installation & Setup

### 1. Configure Your PHP Path (Windows)
The runtime needs to know where your PHP installation directory lives to locate your `.dll` or `.lib` files. 

By default, the runtime checks for Laravel Herd's PHP path (`C:\Users\<user>\.config\herd\bin\php82`). You can easily override this by setting an environment variable in your terminal:

```powershell
$env:RUSTENPHP_PHP_DIR = "C:\Path\To\Your\Php\Installation"