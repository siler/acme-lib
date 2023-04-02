use crate::api::ApiProblem;

pub(crate) type ReqResult<T> = std::result::Result<T, ApiProblem>;

const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

pub(crate) fn req_get(url: &str) -> Result<ureq::Response, ureq::Error> {
    let req = ureq::get(url).timeout(TIMEOUT_DURATION);
    trace!("{:?}", req);
    req.call()
}

pub(crate) fn req_head(url: &str) -> Result<ureq::Response, ureq::Error> {
    let req = ureq::head(url).timeout(TIMEOUT_DURATION);
    trace!("{:?}", req);
    req.call()
}

pub(crate) fn req_post(url: &str, body: &str) -> Result<ureq::Response, ureq::Error> {
    let req = ureq::post(url)
        .set("content-type", "application/jose+json")
        .timeout(TIMEOUT_DURATION);
    trace!("{:?} {}", req, body);
    req.send_string(body)
}

pub(crate) fn req_expect_header(res: &ureq::Response, name: &str) -> ReqResult<String> {
    res.header(name)
        .map(|v| v.to_string())
        .ok_or_else(|| ApiProblem {
            _type: format!("Missing header: {}", name),
            detail: None,
            subproblems: None,
        })
}

pub(crate) fn req_safe_read_body(res: ureq::Response) -> String {
    use std::io::Read;
    let mut res_body = String::new();
    let mut read = res.into_reader();
    // letsencrypt sometimes closes the TLS abruptly causing io error
    // even though we did capture the body.
    read.read_to_string(&mut res_body).ok();
    res_body
}
