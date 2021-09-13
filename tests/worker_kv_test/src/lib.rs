use std::future::Future;

use worker::*;
use worker_kv::{KvError, KvStore};

type TestResult = std::result::Result<String, TestError>;

mod utils;

#[event(fetch)]
pub async fn main(req: Request, env: Env) -> Result<Response> {
    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Create the KV store directly from `worker_kv` as the rust worker sdk uses a published version.
    let store = KvStore::from_this(&env, "test").expect("test kv store not bound");

    Router::new(store)
        .get_async("/get", |req, ctx| wrap(req, ctx, get))
        .get_async("/get-not-found", |req, ctx| wrap(req, ctx, get_not_found))
        .run(req, env)
        .await
}

async fn get(_: Request, ctx: RouteContext<KvStore>) -> TestResult {
    let store = ctx.data().unwrap();
    store
        .get("simple")
        .await
        .map_err(TestError::from)
        .and_then(|v| match v {
            Some(e) => Ok(e.as_string()),
            None => Err(TestError::Other("no value found".into())),
        })
}

async fn get_not_found(_: Request, ctx: RouteContext<KvStore>) -> TestResult {
    let store = ctx.data().unwrap();
    let value = store.get("not_found").await;

    value.map_err(TestError::from).and_then(|v| match v {
        Some(_) => Err(TestError::Other("unexpected value present".into())),
        None => Ok("passed".into()),
    })
}

async fn wrap<T>(
    req: Request,
    ctx: RouteContext<KvStore>,
    func: fn(Request, RouteContext<KvStore>) -> T,
) -> Result<Response>
where
    T: Future<Output = TestResult> + 'static,
{
    let result = func(req, ctx);

    match result.await {
        Ok(value) => Response::ok(value),
        Err(e) => Response::ok(e.to_string()).map(|res| res.with_status(500)),
    }
}

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("{0}")]
    Kv(#[from] KvError),
    #[error("{0}")]
    Other(String),
}
