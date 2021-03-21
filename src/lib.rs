//! Bindings to Cloudflare Worker's [KV](https://developers.cloudflare.com/workers/runtime-apis/kv)
//! to be used ***inside*** of a worker's context.
//!
//! # Example
//! ```no_run
//! let kv = KvStore::create("Example")?;
//! 
//! // Insert a new entry into the kv.
//! kv.put("example_key", "example_value")
//!     .metadata(vec![1, 2, 3, 4]) // Use some arbitrary serialiazable metadata
//!     .execute()
//!     .await?;
//!
//! // NOTE: kv changes can take a minute to become visible to other workers.
//! // Get that same metadata.
//! let (value, metdata) = kv.get_with_metadata::<Vec<usize>>("example_key").await?;
//! ```
#[forbid(missing_docs)]

mod builder;

pub use builder::*;

use js_sys::{global, Function, Object, Promise, Reflect};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// A binding to a Cloudflare KvStore.
#[derive(Clone)]
pub struct KvStore {
    pub(crate) this: Object,
    pub(crate) get_function: Function,
    pub(crate) get_with_meta_function: Function,
    pub(crate) put_function: Function,
    pub(crate) list_function: Function,
    pub(crate) delete_function: Function,
}

impl KvStore {
    /// Creates a new [`KvStore`] with the binding specified in your `wrangler.toml`.
    pub fn create(binding: &str) -> Result<Self, KvError> {
        let this: Object = get(&global(), binding.as_ref())?.into();
        Ok(Self {
            get_function: get(&this, "get")?.into(),
            get_with_meta_function: get(&this, "getWithMetadata")?.into(),
            put_function: get(&this, "put")?.into(),
            list_function: get(&this, "list")?.into(),
            delete_function: get(&this, "delete")?.into(),
            this,
        })
    }

    /// Fetches the value from the kv store by name.
    pub async fn get(&self, name: &str) -> Result<KvValue, KvError> {
        let name = JsValue::from(name);
        let promise: Promise = self.get_function.call1(&self.this, &name)?.into();
        let inner = JsFuture::from(promise)
            .await
            .map_err(KvError::from)?
            .as_string()
            .expect("get request resulted in non-string value");
        Ok(KvValue(inner))
    }

    /// Fetches the value and associated metadata from the kv store by name.
    pub async fn get_with_metadata<M: DeserializeOwned>(
        &self,
        name: &str,
    ) -> Result<(KvValue, M), KvError> {
        let name = JsValue::from(name);
        let promise: Promise = self.get_with_meta_function.call1(&self.this, &name)?.into();
        let pair = JsFuture::from(promise).await?;

        let value = get(&pair, "value")?;
        let metadata = get(&pair, "metadata")?;
        let metadata = metadata
            .as_string()
            .expect("get request resulted in non-string metadata");
        let metadata = serde_json::from_str(&metadata)?;
        let inner = value
            .as_string()
            .expect("get request resulted in non-string value");
        Ok((KvValue(inner), metadata))
    }

    /// Puts data into the kv store.
    pub fn put<T: ToRawKvValue>(&self, name: &str, value: T) -> Result<PutOptionsBuilder, KvError> {
        Ok(PutOptionsBuilder {
            this: self.this.clone(),
            put_function: self.put_function.clone(),
            name: JsValue::from(name),
            value: value.raw_kv_value()?,
            expiration: None,
            expiration_ttl: None,
            metadata: None,
        })
    }

    /// Lists the keys in the kv store.
    pub fn list(&self) -> ListOptionsBuilder {
        ListOptionsBuilder {
            this: self.this.clone(),
            list_function: self.list_function.clone(),
            limit: None,
            cursor: None,
            prefix: None,
        }
    }

    /// Deletes a key in the kv store.
    pub async fn delete(&self, name: &str) -> Result<(), KvError> {
        let name = JsValue::from(name);
        let promise: Promise = self.get_function.call1(&self.this, &name)?.into();
        JsFuture::from(promise).await?;
        Ok(())
    }
}

/// A value fetched via a get request.
#[derive(Debug, Clone)]
pub struct KvValue(String);

impl KvValue {
    /// Gets the value as a string.
    pub fn as_string(self) -> String {
        self.0
    }
    /// Tries to eserialize the inner text to the generic type.
    pub fn as_json<T: DeserializeOwned>(self) -> Result<T, KvError> {
        serde_json::from_str(&self.0).map_err(KvError::from)
    }
    /// Gets the value as a byte slice.
    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        self.0.as_bytes()
    }
}

/// The response for listing the elements in a KV store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    /// A slice of all of the keys in the KV store.
    pub keys: Vec<Key>,
    /// If there are more keys that can be fetched using the response's cursor.
    pub list_complete: bool,
    /// A string used for paginating responses.
    pub cursor: Option<String>,
}

/// The representation of a key in the KV store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    /// The name of the key.
    pub name: String,
    /// When (expressed as a [unix timestamp](https://en.wikipedia.org/wiki/Unix_time)) the key
    /// value pair will expire in the store.
    pub expiration: Option<u64>,
    /// All metadata associated with the key.
    pub metdata: Option<Value>,
}

/// A simple error type that can occur during kv operations.
#[derive(Debug)]
pub enum KvError {
    JavaScript(JsValue),
    Serialization(serde_json::Error),
}

impl Into<JsValue> for KvError {
    fn into(self) -> JsValue {
        match self {
            Self::JavaScript(value) => value,
            Self::Serialization(e) => format!("KvError::Serialization: {}", e.to_string()).into(),
        }
    }
}

impl From<JsValue> for KvError {
    fn from(value: JsValue) -> Self {
        Self::JavaScript(value)
    }
}

impl From<serde_json::Error> for KvError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

/// A trait for things that can be converted to [`wasm_bindgen::JsValue`] to be passed to the kv.
pub trait ToRawKvValue {
    fn raw_kv_value(&self) -> Result<JsValue, KvError>;
}

impl ToRawKvValue for str {
    fn raw_kv_value(&self) -> Result<JsValue, KvError> {
        Ok(JsValue::from(self))
    }
}

impl<T: Serialize> ToRawKvValue for T {
    fn raw_kv_value(&self) -> Result<JsValue, KvError> {
        let serialized = serde_json::to_string(self)?;
        Ok(JsValue::from(serialized))
    }
}

fn get(target: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    Reflect::get(target, &JsValue::from(name))
}
