use std::{
    io::{self, ErrorKind},
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command},
    str::FromStr,
    time::{Duration, Instant},
};

use fs_extra::dir::CopyOptions;
use serde::Deserialize;

#[tokio::test]
async fn integration_test() {
    let mut miniflare_process =
        start_miniflare().expect("unable to spawn miniflare, did you install node modules?");

    wait_for_worker_to_spawn();

    let endpoints = [
        "get",
        "get-not-found",
        "list-keys",
        "put-simple",
        "put-metadata",
        "put-expiration",
    ];

    for endpoint in endpoints {
        let text_res = reqwest::get(&format!("http://localhost:8787/{}", endpoint))
            .await
            .expect("unable to send request")
            .text()
            .await;

        assert!(text_res.is_ok(), "{} failed", endpoint);
        assert_eq!(text_res.unwrap(), "passed".to_string());
    }

    miniflare_process
        .kill()
        .expect("could not kill child process");
}

/// Waits for wrangler to spawn it's http server.
fn wait_for_worker_to_spawn() {
    let now = Instant::now();
    let addr = SocketAddr::from_str("0.0.0.0:8787").unwrap();

    while Instant::now() - now <= Duration::from_secs(5 * 60) {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(_) => return,
            Err(e)
                if e.kind() == ErrorKind::ConnectionRefused
                    || e.kind() == ErrorKind::ConnectionReset => {}
            Err(e) => Err(e).expect("unexpected error connecting to worker"),
        }
    }

    panic!("timed out connecting to worker")
}

fn start_miniflare() -> io::Result<Child> {
    let mf_path = Path::new("tests/worker_kv_test/.mf");

    if mf_path.exists() {
        std::fs::remove_dir_all(mf_path)?;
    }

    fs_extra::dir::copy(
        "tests/worker_kv_test/.mf-init",
        mf_path,
        &CopyOptions {
            content_only: true,
            ..CopyOptions::new()
        },
    )
    .unwrap();

    Command::new("../node_modules/.bin/miniflare")
        .args(&["-c", "wrangler.toml", "-k", "test", "--kv-persist"])
        .current_dir("tests/worker_kv_test")
        .spawn()
}

#[derive(Debug, Deserialize)]
enum TestResult {
    #[serde(rename = "success")]
    Success(String),
    #[serde(rename = "failure")]
    Failure(String),
}
