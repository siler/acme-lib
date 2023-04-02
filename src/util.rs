use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::de::DeserializeOwned;

use crate::{Result, req::ExtractBody};

pub(crate) fn base64url<T: ?Sized + AsRef<[u8]>>(input: &T) -> String {
    URL_SAFE_NO_PAD.encode(input)
}

pub(crate) fn read_json<T: DeserializeOwned>(res: ureq::Response) -> Result<T> {
    let res_body = res.extract_body();
    debug!("{}", res_body);
    Ok(serde_json::from_str(&res_body)?)
}
