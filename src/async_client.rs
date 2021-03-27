use super::api;
use quad_net::http_request::{HttpError, Method, Request, RequestBuilder};

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    JsonError(nanoserde::DeJsonErr),
    HttpError(HttpError),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        Error::IOError(error)
    }
}

impl From<nanoserde::DeJsonErr> for Error {
    fn from(error: nanoserde::DeJsonErr) -> Error {
        Error::JsonError(error)
    }
}

impl From<HttpError> for Error {
    fn from(error: HttpError) -> Error {
        Error::HttpError(error)
    }
}

pub struct AsyncRequest<T: nanoserde::DeJson> {
    _marker: std::marker::PhantomData<T>,
    request: Request,
    on_success: Option<Box<dyn FnMut(T) -> ()>>,
    on_error: Option<Box<dyn FnMut(Error) -> ()>>,
}

pub trait AsyncRequestTick {
    fn tick(&mut self) -> bool;
}

impl<T: nanoserde::DeJson> AsyncRequestTick for AsyncRequest<T> {
    fn tick(&mut self) -> bool {
        match self.try_recv() {
            Some(Ok(response)) => {
                if let Some(on_success) = self.on_success.as_mut() {
                    on_success(response);
                }
                true
            }
            Some(Err(err)) => {
                if let Some(on_error) = self.on_error.as_mut() {
                    on_error(err);
                }
                true
            }
            None => false,
        }
    }
}

impl<T: nanoserde::DeJson> AsyncRequest<T> {
    pub fn on_success<F: FnMut(T) -> () + 'static>(&mut self, f: F) {
        self.on_success = Some(Box::new(f));
    }
    pub fn on_error<F: FnMut(Error) -> () + 'static>(&mut self, f: F) {
        self.on_error = Some(Box::new(f));
    }

    pub fn try_recv(&mut self) -> Option<Result<T, Error>> {
        if let Some(response) = self.request.try_recv() {
            return Some(response.map_err(|err| err.into()).and_then(|response| {
                nanoserde::DeJson::deserialize_json(&response).map_err(|err| err.into())
            }));
        }

        None
    }
}

pub fn make_request<T: nanoserde::DeJson>(
    server: &str,
    port: u32,
    request: api::RestRequest<T>,
) -> AsyncRequest<T> {
    let auth_header = match request.authentication {
        api::Authentication::Basic { username, password } => {
            format!(
                "Basic {}",
                base64::encode(&format!("{}:{}", username, password))
            )
        }
        api::Authentication::Bearer { token } => {
            format!("Bearer {}", token)
        }
    };
    let method = match request.method {
        api::Method::Post => Method::Post,
        api::Method::Put => Method::Put,
        api::Method::Get => Method::Get,
        api::Method::Delete => Method::Delete,
    };

    let url = format!(
        "{}:{}{}?{}",
        server, port, request.urlpath, request.query_params
    );

    let request = RequestBuilder::new(&url)
        .method(method)
        .header("Authorization", &auth_header)
        .body(&request.body)
        .send();

    AsyncRequest {
        request,
        on_success: None,
        on_error: None,
        _marker: std::marker::PhantomData,
    }
}

#[test]
fn auth_async() {
    let request = api::authenticate_email(
        "defaultkey",
        "",
        api::ApiAccountEmail {
            email: "super3@heroes.com".to_string(),
            password: "batsignal2".to_string(),
            vars: std::collections::HashMap::new(),
        },
        Some(false),
        None,
    );

    let mut async_request = make_request("http://127.0.0.1", 7350, request);
    let response = loop {
        if let Some(response) = async_request.try_recv() {
            break response;
        }
    };

    println!("{:?}", response);
}
