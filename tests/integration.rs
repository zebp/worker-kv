use std::{
    io::{self, ErrorKind},
    net::{SocketAddr, TcpStream},
    process::{Child, Command},
    str::FromStr,
    time::{Duration, Instant},
};

use serde::Deserialize;

#[tokio::test]
async fn integration_test() {
    let mut miniflare_process =
        start_miniflare().expect("unable to spawn miniflare, did you install node modules?");

    wait_for_worker_to_spawn();

    miniflare_process
        .kill()
        .expect("could not kill child process");
}

/// Waits for wrangler to spawn it's http server.
fn wait_for_worker_to_spawn() {
    let now = Instant::now();
    let addr = SocketAddr::from_str("0.0.0.0:8787").unwrap();

    while Instant::now() - now <= Duration::from_secs(60) {
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
