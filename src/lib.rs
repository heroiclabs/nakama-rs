mod api_gen;

pub mod api {
    pub use super::api_gen::*;
}

pub use api_gen::ApiClient;

const SERVER_KEY: &str = "defaultKey";
const SERVER_URL: &str = "127.0.0.1";

fn make_rest_request<T: nanoserde::DeJson>(request: api::RestRequest<T>) -> T {
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
    let response: String = ureq::post(&format!(
        "{}{}?{}",
        SERVER_URL, request.urlpath, request.query_params
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
    let api_client = ApiClient::new();
    let request = api_client.authenticate_email(
        SERVER_KEY,
        "",
        api::ApiAccountEmail {
            email: "super@heroes.com".to_string(),
            password: "batsignal".to_string(),
            vars: std::collections::HashMap::new(),
        },
        Some(false),
        None,
    );

    let response = make_rest_request(request);

    println!("{:?}", response);
}
