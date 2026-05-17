use std::{
    env,
    ffi::{c_char, c_int, CString},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    ptr,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetDllDirectoryA(lp_path_name: *const c_char) -> i32;
}

unsafe extern "C" {
    fn php_embed_init(argc: c_int, argv: *mut *mut c_char) -> c_int;
    fn php_embed_shutdown();
    fn zend_eval_string(str: *const c_char, retval_ptr: *mut std::ffi::c_void, string_name: *const c_char) -> c_int;
}

struct PhpRuntime;

impl PhpRuntime {
    fn boot() -> Result<Self, String> {
        configure_windows_php_dll_directory()?;

        let arg0 = CString::new("rustenphp-embed").expect("valid arg0");
        let mut argv = [arg0.as_ptr() as *mut c_char];
        let status = unsafe { php_embed_init(1, argv.as_mut_ptr()) };

        if status != 0 {
            return Err(format!("php_embed_init failed with status {status}"));
        }

        Ok(Self)
    }

    fn eval(&self, php: &str) -> Result<(), String> {
        let code = CString::new(php)
            .map_err(|_| "PHP code contains an interior null byte".to_string())?;
        let name = CString::new("rustenphp eval").expect("valid eval name");
        let status = unsafe { zend_eval_string(code.as_ptr(), ptr::null_mut(), name.as_ptr()) };

        if status == 0 {
            Ok(())
        } else {
            Err(format!("zend_eval_string failed with status {status}"))
        }
    }
}

impl Drop for PhpRuntime {
    fn drop(&mut self) {
        unsafe {
            php_embed_shutdown();
        }
    }
}

fn configure_windows_php_dll_directory() -> Result<(), String> {
    #[cfg(windows)]
    {
        let php_dir = env::var("RUSTENPHP_PHP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"C:\Users\korom\.config\herd\bin\php82"));
        let php_dir = CString::new(php_dir.to_string_lossy().as_bytes())
            .map_err(|_| "PHP directory path contains an interior null byte".to_string())?;
        let status = unsafe { SetDllDirectoryA(php_dir.as_ptr()) };

        if status == 0 {
            return Err("SetDllDirectoryA failed for the PHP runtime directory".to_string());
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.first().is_some_and(|arg| arg == "serve") {
        return serve_laravel();
    }

    let php = args.join(" ");
    let php = if php.trim().is_empty() {
        "echo 'PHP embedded in Rust via php8embed.lib: '.PHP_VERSION.PHP_EOL;".to_string()
    } else {
        php
    };

    let runtime = PhpRuntime::boot()?;
    runtime.eval(&php)?;

    Ok(())
}

fn serve_laravel() -> Result<(), String> {
    let laravel_root = env::var("RUSTENPHP_LARAVEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(r"C:\Users\korom\Documents\Codex\2026-05-16\i-want-you-to-create-a\lettings-mvp")
        });
    let public_root = laravel_root.join("public");
    let host = env::var("RUSTENPHP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("RUSTENPHP_PORT").unwrap_or_else(|_| "8787".to_string());
    let address = format!("{host}:{port}");
    let listener = TcpListener::bind(&address)
        .map_err(|error| format!("failed to bind {address}: {error}"))?;

    println!("rustenphp embedded Laravel server listening on http://{address}");
    println!("Laravel root: {}", laravel_root.display());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_connection(&mut stream, &laravel_root, &public_root, &host, &port) {
                    let body = format!("rustenphp error: {error}");
                    let _ = write_response(&mut stream, 500, "text/plain; charset=utf-8", body.as_bytes());
                }
            }
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(
    stream: &mut TcpStream,
    laravel_root: &Path,
    public_root: &Path,
    host: &str,
    port: &str,
) -> Result<(), String> {
    let request = read_http_request(stream)?;
    let Some(request_line) = request.lines().next() else {
        return Err("empty HTTP request".to_string());
    };

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let target = parts.next().unwrap_or("/");
    let (path, query) = split_target(target);

    if method != "GET" && method != "HEAD" {
        return write_response(
            stream,
            405,
            "text/plain; charset=utf-8",
            b"Only GET and HEAD are implemented in this prototype.",
        );
    }

    if let Some(static_file) = resolve_static_file(public_root, path) {
        let body = fs::read(&static_file)
            .map_err(|error| format!("failed to read static file {}: {error}", static_file.display()))?;
        return write_response(stream, 200, content_type(&static_file), &body);
    }

    let body = run_laravel_request(laravel_root, public_root, method, target, path, query, host, port)?;
    write_response(stream, 200, "text/html; charset=utf-8", body.as_bytes())
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut buffer = [0_u8; 8192];
    let bytes_read = stream
        .read(&mut buffer)
        .map_err(|error| format!("failed to read request: {error}"))?;

    Ok(String::from_utf8_lossy(&buffer[..bytes_read]).to_string())
}

fn split_target(target: &str) -> (&str, &str) {
    target
        .split_once('?')
        .map_or((target, ""), |(path, query)| (path, query))
}

fn resolve_static_file(public_root: &Path, path: &str) -> Option<PathBuf> {
    let path = path.trim_start_matches('/').replace('\\', "/");

    if path.is_empty() || path.contains("..") {
        return None;
    }

    let candidate = public_root.join(path);

    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn run_laravel_request(
    laravel_root: &Path,
    public_root: &Path,
    method: &str,
    target: &str,
    _path: &str,
    query: &str,
    host: &str,
    port: &str,
) -> Result<String, String> {
    let output_path = env::temp_dir().join(format!("rustenphp-response-{}.html", unique_id()));
    let index_path = public_root.join("index.php");

    env::set_current_dir(laravel_root)
        .map_err(|error| format!("failed to chdir to Laravel root {}: {error}", laravel_root.display()))?;

    let php = format!(
        r#"
$__rustenphp_output_path = {output_path};
error_reporting(E_ALL);
ini_set('display_errors', '1');
register_shutdown_function(function () use ($__rustenphp_output_path) {{
    $error = error_get_last();
    if ($error && !file_exists($__rustenphp_output_path)) {{
        file_put_contents(
            $__rustenphp_output_path,
            "PHP fatal error: ".$error['message']." in ".$error['file'].":".$error['line']
        );
    }}
}});

try {{
    $_SERVER['REQUEST_METHOD'] = {method};
    $_SERVER['REQUEST_URI'] = {target};
    $_SERVER['QUERY_STRING'] = {query};
    $_SERVER['SCRIPT_FILENAME'] = {script_filename};
    $_SERVER['SCRIPT_NAME'] = '/index.php';
    $_SERVER['PHP_SELF'] = '/index.php';
    $_SERVER['DOCUMENT_ROOT'] = {document_root};
    $_SERVER['SERVER_NAME'] = {host};
    $_SERVER['SERVER_PORT'] = {port};
    $_SERVER['SERVER_PROTOCOL'] = 'HTTP/1.1';
    $_SERVER['HTTP_HOST'] = {http_host};
    $_SERVER['HTTPS'] = 'off';
    $_GET = [];
    parse_str({query}, $_GET);
    $_POST = [];
    $_REQUEST = $_GET;
    ob_start();
    require {index_path};
    $__rustenphp_body = ob_get_clean();
    file_put_contents($__rustenphp_output_path, $__rustenphp_body);
}} catch (Throwable $exception) {{
    while (ob_get_level() > 0) {{
        ob_end_clean();
    }}
    file_put_contents(
        $__rustenphp_output_path,
        "PHP throwable: ".$exception::class.": ".$exception->getMessage()." in ".$exception->getFile().":".$exception->getLine()
    );
}}
"#,
        method = php_string(method),
        target = php_string(target),
        query = php_string(query),
        host = php_string(host),
        port = php_string(port),
        http_host = php_string(&format!("{host}:{port}")),
        script_filename = php_string(&index_path.to_string_lossy()),
        document_root = php_string(&public_root.to_string_lossy()),
        index_path = php_string(&index_path.to_string_lossy()),
        output_path = php_string(&output_path.to_string_lossy()),
    );

    {
        let runtime = PhpRuntime::boot()?;
        runtime.eval(&php)?;
    }

    let body = fs::read_to_string(&output_path)
        .map_err(|error| format!("failed to read PHP response {}: {error}", output_path.display()))?;
    let _ = fs::remove_file(output_path);

    Ok(body)
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("failed to write response: {error}"))
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()).unwrap_or("") {
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn php_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\r', "\\r")
        .replace('\n', "\\n");

    format!("'{escaped}'")
}

fn unique_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_nanos()
}
