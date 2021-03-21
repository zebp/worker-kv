use js_sys::{Function, Object, Promise, JSON};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

use crate::{KvError, ListResponse};

/// A builder to configure put requests.
#[derive(Debug, Clone, Serialize)]
pub struct PutOptionsBuilder {
    #[serde(skip)]
    pub(crate) this: Object,
    #[serde(skip)]
    pub(crate) put_function: Function,
    #[serde(skip)]
    pub(crate) name: JsValue,
    #[serde(skip)]
    pub(crate) value: JsValue,
    pub(crate) expiration: Option<u64>,
    #[serde(rename = "expirationTtl")]
    pub(crate) expiration_ttl: Option<u64>,
    pub(crate) metadata: Option<Value>,
}

impl PutOptionsBuilder {
    /// When (expressed as a [unix timestamp](https://en.wikipedia.org/wiki/Unix_time)) the key
    /// value pair will expire in the store.
    pub fn expiration(mut self, expiration: u64) -> Self {
        self.expiration = Some(expiration);
        self
    }
    /// How many seconds until the key value pair will expire.
    pub fn expiration_ttl(mut self, expiration_ttl: u64) -> Self {
        self.expiration_ttl = Some(expiration_ttl);
        self
    }
    /// Metadata to be stored with the key value pair.
    pub fn metadata<T: Serialize>(mut self, metadata: T) -> Result<Self, KvError> {
        self.metadata = Some(serde_json::to_value(metadata)?);
        Ok(self)
    }
    /// Puts the value in the kv store.
    pub async fn execute(self) -> Result<(), KvError> {
        let options_string = serde_json::to_string(&self)?;
        let options_object = JSON::parse(&options_string)?;

        let promise: Promise = self
            .put_function
            .call3(&self.this, &self.name, &self.value, &options_object)?
            .into();
        JsFuture::from(promise)
            .await
            .map(|_| ())
            .map_err(KvError::from)
    }
}

/// A builder to configure list requests.
#[derive(Debug, Clone, Serialize)]
pub struct ListOptionsBuilder {
    #[serde(skip)]
    pub(crate) this: Object,
    #[serde(skip)]
    pub(crate) list_function: Function,
    pub(crate) limit: Option<u64>,
    pub(crate) cursor: Option<String>,
    pub(crate) prefix: Option<String>,
}

impl ListOptionsBuilder {
    /// The maximum number of keys returned. The default is 1000, which is the maximum. It is
    /// unlikely that you will want to change this default, but it is included for completeness.
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    /// A string returned by a previous response used to paginate the keys in the store.
    pub fn cursor(mut self, cursor: String) -> Self {
        self.cursor = Some(cursor);
        self
    }
    /// A prefix that all keys must start with for them to be included in the response.
    pub fn prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }
    /// Lists the key value pairs in the kv store.
    pub async fn execute(self) -> Result<ListResponse, KvError> {
        let options_string = serde_json::to_string(&self)?;
        let options_object = JSON::parse(&options_string)?;

        let promise: Promise = self
            .list_function
            .call1(&self.this, &options_object)?
            .into();
        let json_value = JSON::stringify(&JsFuture::from(promise).await?)?
            .as_string()
            .unwrap();
        serde_json::from_str(&json_value).map_err(KvError::from)
    }
}
