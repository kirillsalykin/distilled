#[cfg(feature = "derive")]
pub use distilled_derive::Distilled;

use serde::{Deserialize, Serialize};
// use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashMap;
use validator;

pub trait Distilled: Sized {
    fn distill<'a, T: Into<Option<&'a Value>>>(value: T) -> Result<Self, Error>;
}

impl Distilled for () {
    fn distill<'a, T: Into<Option<&'a Value>>>(value: T) -> Result<Self, Error> {
        match value.into() {
            None => Ok(()),
            Some(v) if v.is_null() => Ok(()),
            _ => Err(Error::entry("wrong_type")),
        }
    }
}

impl Distilled for String {
    fn distill<'a, T: Into<Option<&'a Value>>>(value: T) -> Result<Self, Error> {
        let value = value.into().ok_or(Error::entry("missing_field"))?;
        value
            .as_str()
            .map(String::from)
            .ok_or(Error::entry("wrong_type"))
    }
}

impl Distilled for u32 {
    fn distill<'a, T: Into<Option<&'a Value>>>(value: T) -> Result<Self, Error> {
        let value = value.into().ok_or(Error::entry("missing_field"))?;
        let n = value.as_i64().ok_or(Error::entry("wrong_type"))?;
        u32::try_from(n).map_err(|_| Error::entry("wrong_type"))
    }
}

impl<T: Distilled> Distilled for Option<T> {
    fn distill<'a, U: Into<Option<&'a Value>>>(value: U) -> Result<Self, Error> {
        match value.into() {
            Some(v) => Ok(Some(T::distill(v)?)),
            None => Ok(None),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub code: Cow<'static, str>,
    // pub message: Option<Cow<'static, str>>,
    pub params: HashMap<Cow<'static, str>, Value>,
}

impl ErrorEntry {
    pub fn new(code: &'static str) -> Self {
        Self {
            code: Cow::from(code),
            // message: None,
            params: HashMap::new(),
        }
    }
}

// impl std::error::Error for Error {
//     fn description(&self) -> &str {
//         &self.code
//     }
//     fn cause(&self) -> Option<&dyn std::error::Error> {
//         None
//     }
// }

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum Error {
    Struct(HashMap<Cow<'static, str>, Error>),
    List(BTreeMap<usize, Box<Error>>),
    Entry(ErrorEntry),
}

impl Error {
    pub fn entry(code: &'static str) -> Self {
        Error::Entry(ErrorEntry::new(code))
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ErrorMap(pub HashMap<Cow<'static, str>, Error>);
