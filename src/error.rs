//
use std::{fmt, io};

use crate::{api::ApiProblem, req::ExtractBody};

/// acme-lib result.
pub type Result<T> = std::result::Result<T, Error>;

/// acme-lib errors.
#[derive(Debug)]
pub enum Error {
    /// An API call failed.
    ApiProblem(ApiProblem),
    /// An API call failed.
    Call(String),
    /// Base64 decoding failed.
    Base64Decode(base64::DecodeError),
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// std::io error.
    Io(io::Error),
    /// Some other error. Notice that `Error` is
    /// `From<String>` and `From<&str>` and it becomes `Other`.
    Other(String),
}
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ApiProblem(a) => write!(f, "{}", a),
            Error::Call(s) => write!(f, "{}", s),
            Error::Base64Decode(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<ApiProblem> for Error {
    fn from(e: ApiProblem) -> Self {
        Error::ApiProblem(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

impl From<ureq::Error> for Error {
    fn from(value: ureq::Error) -> Self {
        let res = match value {
            ureq::Error::Status(_, res) => res,
            ureq::Error::Transport(_) => {
                return Error::ApiProblem(ApiProblem {
                    _type: "httpReqError".into(),
                    detail: Some("Transport error".into()),
                    subproblems: None,
                })
            }
        };

        let problem = if res.content_type() == "application/problem+json" {
            // if we were sent a problem+json, deserialize it
            let body = res.extract_body();
            serde_json::from_str(&body).unwrap_or_else(|e| ApiProblem {
                _type: "problemJsonFail".into(),
                detail: Some(format!(
                    "Failed to deserialize application/problem+json ({}) body: {}",
                    e, body
                )),
                subproblems: None,
            })
        } else {
            // some other problem
            let status = format!("{} {}", res.status(), res.status_text());
            let body = res.extract_body();
            let detail = format!("{} body: {}", status, body);

            ApiProblem {
                _type: "httpReqError".into(),
                detail: Some(detail),
                subproblems: None,
            }
        };

        Error::ApiProblem(problem)
    }
}

impl From<Box<ureq::Error>> for Error {
    fn from(value: Box<ureq::Error>) -> Self {
        Error::from(*value)
    }
}
