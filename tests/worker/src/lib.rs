mod utils;

use std::{fmt::Debug, future::Future, pin::Pin};

use js_sys::Promise;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;
use worker_kv::*;

/// Step one clears the store and inserts values to be checked in step two.
async fn step_one(kv: KvStore) -> Result<(), KvError> {
    let list_res = kv.list().execute().await?;

    // If there are more than 1000 entries we know that the kv is in an invalid state.
    if !list_res.list_complete {
        return Err(make_kv_error("list is incomplete, more than 1000 entries"));
    }

    // Clear the kv store by deleting the first 1000 keys, which should be all of them.
    for key in list_res.keys {
        kv.delete(&key.name).await?;
    }

    // Insert a value with no metadata or expiration.
    kv.put("a", "b")?.execute().await?;
    // Insert a value without any metadata that will expire in 10 minutes.
    kv.put("b", "c")?.expiration_ttl(10 * 60).execute().await?;
    // Insert a value with some metadata.
    kv.put("c", "d")?.metadata(10)?.execute().await?;
    // Insert a value with some metadata that will expire in 10 minutes.
    kv.put("d", "e")?
        .metadata(20)?
        .expiration_ttl(10 * 60)
        .execute()
        .await?;

    Ok(())
}

/// Step two checks to see all the keys created in step one exist and that their values are
/// consistent with the expected values.
async fn step_two(kv: KvStore) -> Result<(), KvError> {
    let res = kv.list().execute().await?;

    // If there are more than 1000 entries we know that the kv is in an invalid state.
    if !res.list_complete {
        return Err(make_kv_error("list is incomplete, more than 1000 entries"));
    }

    let keys = &res.keys;

    // Ensures all the keys are in the list respone.
    check_key::<()>(keys, "a", None, false)?;
    check_key::<()>(keys, "b", None, true)?;
    check_key::<u8>(keys, "c", Some(10), false)?;
    check_key::<u8>(keys, "d", Some(20), true)?;

    // Checks the values for keys that shouldn't return any metadata.
    check_value(kv.get("a").await?, "b", "a")?;
    check_value(kv.get("b").await?, "c", "b")?;

    // Checks the values and metadata for keys that should return some metadata.
    check_value_and_metadata(kv.get_with_metadata("c").await?, "d", 10, "c")?;
    check_value_and_metadata(kv.get_with_metadata("d").await?, "e", 20, "d")?;

    Ok(())
}

#[wasm_bindgen(js_name = "runStep")]
pub fn run_step(step: usize) -> Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        let kv = KvStore::create("EXAMPLE").unwrap();
        let future: Pin<Box<dyn Future<Output = Result<(), KvError>>>> = match step {
            0 => Box::pin(step_one(kv)),
            1 => Box::pin(step_two(kv)),
            _ => Box::pin(async { Err(make_kv_error("invalid step")) }),
        };

        future
            .await
            .map(|_| JsValue::UNDEFINED)
            .map_err(KvError::into)
    })
}

fn make_kv_error(reason: &str) -> KvError {
    let error = serde_json::json!({ "error": reason });
    let error = JsValue::from_serde(&error).unwrap();
    KvError::JavaScript(error)
}

fn check_key<T: DeserializeOwned + Eq + Debug>(
    keys: &[Key],
    name: &str,
    metadata: Option<T>,
    expiration_expected: bool,
) -> Result<(), KvError> {
    let key = keys
        .iter()
        .find(|key| key.name == name)
        .ok_or_else(|| make_kv_error("key not found"))?;

    let deserialized_meta: Option<T> = key
        .metadata
        .clone()
        .map(|value| serde_json::from_value(value).unwrap());
    if deserialized_meta != metadata {
        return Err(make_kv_error(&format!(
            "metadata doesn't match for {}",
            name
        )))
    }

    match (key.expiration.is_some(), expiration_expected) {
        (false, true) => Err(make_kv_error("expected expiration timestamp not found")),
        (true, false) => Err(make_kv_error(
            "expiration timestamp found when not expected",
        )),
        _ => Ok(()),
    }
}

fn check_value(kv_value: Option<KvValue>, expected_value: &str, name: &str) -> Result<(), KvError> {
    let kv_value = kv_value.ok_or_else(|| make_kv_error(&format!("{} not present", name)))?;
    let value = kv_value.as_string();

    if value != expected_value {
        return Err(make_kv_error(&format!("{} had unexpected value", name)));
    }

    Ok(())
}

fn check_value_and_metadata<M>(
    kv_value: Option<(KvValue, M)>,
    expected_value: &str,
    expected_metadata: M,
    name: &str,
) -> Result<(), KvError>
where
    M: DeserializeOwned + Eq,
{
    let (kv_value, metadata) =
        kv_value.ok_or_else(|| make_kv_error(&format!("{} not present", name)))?;
    let value = kv_value.as_string();

    if value != expected_value {
        return Err(make_kv_error(&format!("{} had unexpected value", name)));
    }

    if metadata != expected_metadata {
        return Err(make_kv_error(&format!("{} had unexpected metadata", name)));
    }

    Ok(())
}
