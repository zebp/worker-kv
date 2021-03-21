use std::collections::HashMap;

use js_sys::{global, Function, Object, Promise, Reflect, JSON};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

#[derive(Clone)]
pub struct KV {
    this: Object,
    get_function: Function,
    put_function: Function,
    list_function: Function,
}

impl KV {
    pub fn new(name: impl AsRef<str>) -> Result<Self, KvError> {
        let name = JsValue::from(name.as_ref());
        let this: Object = Reflect::get(&global(), &name)?.into();
        Ok(Self {
            get_function: Reflect::get(&this, &JsValue::from("get"))?.into(),
            put_function: Reflect::get(&this, &JsValue::from("put"))?.into(),
            list_function: Reflect::get(&this, &JsValue::from("list"))?.into(),
            this,
        })
    }

    pub async fn get(&self, name: impl AsRef<str>) -> Result<JsValue, KvError> {
        let name = JsValue::from(name.as_ref());
        let promise: Promise = self.get_function.call1(&self.this, &name)?.into();
        JsFuture::from(promise).await.map_err(KvError::from)
    }

    pub async fn put<T: KvValue>(
        &self,
        name: impl AsRef<str>,
        value: T,
    ) -> Result<JsValue, KvError> {
        let name = JsValue::from(name.as_ref());
        let promise: Promise = self
            .put_function
            .call2(&self.this, &name, &value.raw_kv_value())?
            .into();
        JsFuture::from(promise).await.map_err(KvError::from)
    }

    pub async fn list(&self) -> Result<ListResponse, KvError> {
        self.list_with_options(ListOptions::default()).await
    }

    pub async fn list_with_options(&self, options: ListOptions) -> Result<ListResponse, KvError> {
        let options_string = serde_json::to_string(&options)?;
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

/// Optional information that can be used when listing keys in the store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListOptions {
    /// The maximum number of keys returned. The default is 1000, which is the maximum. It is
    /// unlikely that you will want to change this default, but it is included for completeness.
    limit: Option<u64>,
    /// A string returned by a previous response used to paginate the keys in the store.
    cursor: Option<String>,
    /// A prefix that all keys must start with for them to be included in the response.
    prefix: Option<String>,
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
    /// value pair will expire in the database.
    pub expiration: Option<u64>,
    /// All metadata associated with the key.
    pub metdata: Option<HashMap<String, Value>>,
}

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

pub trait KvValue {
    fn raw_kv_value(&self) -> JsValue;
}

impl KvValue for str {
    fn raw_kv_value(&self) -> JsValue {
        JsValue::from(self)
    }
}
