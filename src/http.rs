use std::env;
use hyper::client::{Client, RedirectPolicy};
use hyper::header::{Connection, Authorization, Basic, UserAgent};
use hyper::status::StatusCode;
use serde_json as json;
use serde;
use error::{Error, Result};
use std::io::Read;

pub struct HttpClient {
    client: Client,
    authorization: Option<Authorization<Basic>>,
}

impl HttpClient {
    pub fn new() -> HttpClient {
        let mut client = match env::var("HTTP_PROXY") {
            Ok(mut proxy) => {
                let mut port = 80;
                if let Some(colon) = proxy.rfind(':') {
                    port = proxy[colon + 1..].parse().expect("$HTTP_PROXY is invalid");
                    proxy.truncate(colon);
                }
                Client::with_http_proxy(proxy, port)
            }
            _ => Client::new(),
        };

        client.set_redirect_policy(RedirectPolicy::FollowAll);

        HttpClient {
            client: client,
            authorization: None,
        }
    }

    pub fn with_basic_authorization<U, P>(&mut self, username: U, password: P) -> &mut Self
        where U: Into<String>,
              P: Into<String>
    {
        self.authorization = Some(Authorization(Basic {
            username: username.into(),
            password: Some(password.into()),
        }));
        self
    }

    pub fn post_object<S, D>(self, url: &str, payload: &S) -> Result<D>
        where S: serde::ser::Serialize,
              D: serde::de::Deserialize
    {
        let mut res = try!(if let Some(auth) = self.authorization {
            self.client
                .post(url)
                .header(Connection::close())
                .header(UserAgent(format!("{}/{}", "create_gh_repo", crate_version!())))
                .header(auth)
                .body(try!(json::to_string(payload)).as_bytes())
                .send()
        } else {
            self.client
                .post(url)
                .header(Connection::close())
                .header(UserAgent(format!("{}/{}", "create_gh_repo", crate_version!())))
                .body(try!(json::to_string(payload)).as_bytes())
                .send()
        });
        let mut res_body = String::new();
        let _ = res.read_to_string(&mut res_body);

        match json::from_str(&res_body) {
            Ok(r) => Ok(r),
            Err(e) => {
                error!("{}", res_body);
                Err(e.into())
            },
        }
    }
}
