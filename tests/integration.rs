use std::{
    io::{self, ErrorKind},
    net::{SocketAddr, TcpStream},
    process::{Child, Command},
    str::FromStr,
    time::{Duration, Instant},
};

use serde::Deserialize;

/// The duration slept to allow for the changes to the kv store to propagate.
const SLEEP_DURATION: Duration = Duration::from_secs(60);

#[tokio::test]
async fn integration_test() {
    let account_id = std::env::var("ACCOUNT_ID").expect("ACCOUNT_ID not specified");
    let kv_id = std::env::var("KV_ID").expect("KV_ID not specified");
    let mut wrangler_process =
        spawn_wrangler(&account_id, &kv_id).expect("unable to write wrangler config");

    wait_for_worker_to_spawn();

    let resp: Response = reqwest::get("http://127.0.0.1:8787/0")
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    match resp {
        Response::Successful { success } => {
            assert!(success, "step one request failed");
        }
        Response::Error { error } => {
            panic!("{}", error);
        }
    }

    // Sleep a minute to allow for kv changes to propagate.
    tokio::time::sleep(SLEEP_DURATION).await;

    let resp: Response = reqwest::get("http://127.0.0.1:8787/1")
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    match resp {
        Response::Successful { success } => {
            assert!(success, "step one request failed");
        }
        Response::Error { error } => {
            panic!("{}", error);
        }
    }

    wrangler_process
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
            Err(e) if e.kind() == ErrorKind::ConnectionRefused => {}
            Err(e) => Err(e).expect("unexpected error connecting to worker"),
        }
    }

    panic!("timed out connecting to worker")
}

/// Formats the wrangler template and writes it to the test worker.
fn spawn_wrangler(account_id: &str, kv_id: &str) -> io::Result<Child> {
    let contents = include_str!("wrangler.template.toml")
        .replace("ACCOUNT_ID", account_id)
        .replace("KV_ID", kv_id);
    std::fs::write("tests/worker/wrangler.toml", contents)?;

    Command::new("wrangler")
        .arg("dev")
        .current_dir("tests/worker")
        .spawn()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Response {
    Successful { success: bool },
    Error { error: String },
}
