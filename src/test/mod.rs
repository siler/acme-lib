#![allow(clippy::trivial_regex)]

use futures::TryFutureExt;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use once_cell::sync::Lazy;
use std::{convert::Infallible, net::TcpListener, sync::Arc};
use tokio::runtime::Runtime;

static RE_URL: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new("<URL>").unwrap());

pub struct TestServer {
    _rt: Runtime,
    pub dir_url: String,
}

fn get_directory(url: &str) -> Response<Body> {
    const BODY: &str = r#"{
    "keyChange": "<URL>/acme/key-change",
    "newAccount": "<URL>/acme/new-acct",
    "newNonce": "<URL>/acme/new-nonce",
    "newOrder": "<URL>/acme/new-order",
    "revokeCert": "<URL>/acme/revoke-cert",
    "meta": {
        "caaIdentities": [
        "testdir.org"
        ]
    }
    }"#;
    Response::new(Body::from(RE_URL.replace_all(BODY, url)))
}

fn head_new_nonce() -> Response<Body> {
    Response::builder()
        .status(204)
        .header(
            "Replay-Nonce",
            "8_uBBV3N2DBRJczhoiB46ugJKUkUHxGzVe6xIMpjHFM",
        )
        .body(Body::empty())
        .unwrap()
}

fn post_new_acct(url: &str) -> Response<Body> {
    const BODY: &str = r#"{
    "id": 7728515,
    "key": {
        "use": "sig",
        "kty": "EC",
        "crv": "P-256",
        "alg": "ES256",
        "x": "ttpobTRK2bw7ttGBESRO7Nb23mbIRfnRZwunL1W6wRI",
        "y": "h2Z00J37_2qRKH0-flrHEsH0xbit915Tyvd2v_CAOSk"
    },
    "contact": [
        "mailto:foo@bar.com"
    ],
    "initialIp": "90.171.37.12",
    "createdAt": "2018-12-31T17:15:40.399104457Z",
    "status": "valid"
    }"#;
    let location: String = RE_URL.replace_all("<URL>/acme/acct/7728515", url).into();
    Response::builder()
        .status(201)
        .header("Location", location)
        .body(Body::from(BODY))
        .unwrap()
}

fn post_new_order(url: &str) -> Response<Body> {
    const BODY: &str = r#"{
    "status": "pending",
    "expires": "2019-01-09T08:26:43.570360537Z",
    "identifiers": [
        {
        "type": "dns",
        "value": "acmetest.example.com"
        }
    ],
    "authorizations": [
        "<URL>/acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs"
    ],
    "finalize": "<URL>/acme/finalize/7738992/18234324"
    }"#;
    let location: String = RE_URL
        .replace_all("<URL>/acme/order/YTqpYUthlVfwBncUufE8", url)
        .into();
    Response::builder()
        .status(201)
        .header("Location", location)
        .body(Body::from(RE_URL.replace_all(BODY, url)))
        .unwrap()
}

fn post_get_order(url: &str) -> Response<Body> {
    const BODY: &str = r#"{
    "status": "<STATUS>",
    "expires": "2019-01-09T08:26:43.570360537Z",
    "identifiers": [
        {
        "type": "dns",
        "value": "acmetest.example.com"
        }
    ],
    "authorizations": [
        "<URL>/acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs"
    ],
    "finalize": "<URL>/acme/finalize/7738992/18234324",
    "certificate": "<URL>/acme/cert/fae41c070f967713109028"
    }"#;
    let b = RE_URL.replace_all(BODY, url).to_string();
    Response::builder().status(200).body(Body::from(b)).unwrap()
}

fn post_authz(url: &str) -> Response<Body> {
    const BODY: &str = r#"{
        "identifier": {
            "type": "dns",
            "value": "acmetest.algesten.se"
        },
        "status": "pending",
        "expires": "2019-01-09T08:26:43Z",
        "challenges": [
        {
            "type": "http-01",
            "status": "pending",
            "url": "<URL>/acme/challenge/YTqpYUthlVfwBncUufE8IRWLMSRqcSs/216789597",
            "token": "MUi-gqeOJdRkSb_YR2eaMxQBqf6al8dgt_dOttSWb0w"
        },
        {
            "type": "tls-alpn-01",
            "status": "pending",
            "url": "<URL>/acme/challenge/YTqpYUthlVfwBncUufE8IRWLMSRqcSs/216789598",
            "token": "WCdRWkCy4THTD_j5IH4ISAzr59lFIg5wzYmKxuOJ1lU"
        },
        {
            "type": "dns-01",
            "status": "pending",
            "url": "<URL>/acme/challenge/YTqpYUthlVfwBncUufE8IRWLMSRqcSs/216789599",
            "token": "RRo2ZcXAEqxKvMH8RGcATjSK1KknLEUmauwfQ5i3gG8"
        }
        ]
    }"#;
    Response::builder()
        .status(201)
        .body(Body::from(RE_URL.replace_all(BODY, url)))
        .unwrap()
}

fn post_finalize(_url: &str) -> Response<Body> {
    Response::builder().status(200).body(Body::empty()).unwrap()
}

fn post_certificate(_url: &str) -> Response<Body> {
    Response::builder()
        .status(200)
        .body("CERT HERE".into())
        .unwrap()
}

fn route_request(req: Request<Body>, uri: &str) -> Response<Body> {
    let method = req.method();
    let path = req.uri().path();

    match (method, path) {
        (&Method::GET, "/directory") => get_directory(uri),
        (&Method::HEAD, "/acme/new-nonce") => head_new_nonce(),
        (&Method::POST, "/acme/new-acct") => post_new_acct(uri),
        (&Method::POST, "/acme/new-order") => post_new_order(uri),
        (&Method::POST, "/acme/order/YTqpYUthlVfwBncUufE8") => post_get_order(uri),
        (&Method::POST, "/acme/authz/YTqpYUthlVfwBncUufE8IRWLMSRqcSs") => post_authz(uri),
        (&Method::POST, "/acme/finalize/7738992/18234324") => post_finalize(uri),
        (&Method::POST, "/acme/cert/fae41c070f967713109028") => post_certificate(uri),
        (_, _) => Response::builder().status(404).body(Body::empty()).unwrap(),
    }
}

pub fn with_directory_server() -> TestServer {
    println!("creating server");
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = tcp.local_addr().unwrap().port();

    let url = format!("http://127.0.0.1:{}", port);
    let arc_url = Arc::new(url);
    let dir_url = format!("{}/directory", arc_url.as_str());

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.spawn(async move {
        println!("entered spawned fn");
        let make_service = make_service_fn(move |_conn| {
            let svc_url = arc_url.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let req_url = svc_url.clone();
                    async move { Ok::<_, Infallible>(route_request(req, req_url.as_str())) }
                }))
            }
        });

        println!("preparing server");
        let result = Server::from_tcp(tcp)
            .unwrap()
            .serve(make_service)
            .map_err(|err| eprintln!("server error: {}", err));

        println!("starting server");
        result.await
    });

    println!("returning server");

    TestServer { _rt: rt, dir_url }
}

#[test]
pub fn test_make_directory() {
    let server = with_directory_server();
    println!("sending request");
    let res = ureq::get(&server.dir_url).call();
    println!("received request response");
    println!("{:?}", res);
    assert!(res.is_ok());
}
