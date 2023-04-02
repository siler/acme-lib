use crate::api::ApiProblem;

pub(crate) type ApiResult<T> = std::result::Result<T, ApiProblem>;

const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

pub(crate) fn get(url: &str) -> Result<ureq::Response, Box<ureq::Error>> {
    let req = ureq::get(url).timeout(TIMEOUT_DURATION);
    trace!("{:?}", req);
    req.call().map_err(Box::new)
}

pub(crate) fn head(url: &str) -> Result<ureq::Response, Box<ureq::Error>> {
    let req = ureq::head(url).timeout(TIMEOUT_DURATION);
    trace!("{:?}", req);
    req.call().map_err(Box::new)
}

pub(crate) fn post(url: &str, body: &str) -> Result<ureq::Response, Box<ureq::Error>> {
    let req = ureq::post(url)
        .set("content-type", "application/jose+json")
        .timeout(TIMEOUT_DURATION);
    trace!("{:?} {}", req, body);
    req.send_string(body).map_err(Box::new)
}

pub(crate) trait ExtractHeader {
    fn extract_header(&self, name: &str) -> ApiResult<String>;
}

impl ExtractHeader for ureq::Response {
    fn extract_header(&self, name: &str) -> ApiResult<String> {
        self.header(name)
            .map(|v| v.to_string())
            .ok_or_else(|| ApiProblem {
                _type: format!("Missing header: {}", name),
                detail: None,
                subproblems: None,
            })
    }
}

pub(crate) trait ExtractBody {
    fn extract_body(self) -> String;
}

impl ExtractBody for ureq::Response {
    fn extract_body(self) -> String {
        use std::io::Read;

        let mut res_body = String::new();
        // letsencrypt sometimes closes the TLS abruptly causing io error
        // even though we did capture the body.
        self.into_reader().read_to_string(&mut res_body).ok();
        res_body
    }
}
