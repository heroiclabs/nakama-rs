mod api_gen;

pub mod api {
    pub use super::api_gen::*;
}

pub mod sync_client {
    use super::api;

    pub fn make_request<T: nanoserde::DeJson>(
        server: &str,
        port: u32,
        request: api::RestRequest<T>,
    ) -> T {
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
            api::Method::Post => ureq::post,
            api::Method::Put => ureq::put,
            api::Method::Get => ureq::get,
            api::Method::Delete => ureq::delete,
        };

        let response: String = method(&format!(
            "{}:{}{}?{}",
            server, port, request.urlpath, request.query_params
        ))
        .set("Authorization", &auth_header)
        .send_string(&request.body)
        .unwrap()
        .into_string()
        .unwrap();

        nanoserde::DeJson::deserialize_json(&response).unwrap()
    }

    #[test]
    fn auth() {
        let request = api::authenticate_email(
            "defaultKey",
            "",
            api::ApiAccountEmail {
                email: "super@heroes.com".to_string(),
                password: "batsignal".to_string(),
                vars: std::collections::HashMap::new(),
            },
            Some(false),
            None,
        );

        let response = make_request("http://127.0.0.1", 7350, request);

        println!("{:?}", response);
    }
}
