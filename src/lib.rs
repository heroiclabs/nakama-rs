mod api_gen;

pub mod api {
    pub use super::api_gen::*;
}

pub mod rt_api {
    use nanoserde::DeJson;
    use quad_net::web_socket::WebSocket;

    #[derive(DeJson, Debug)]
    pub struct EventContainer {
        /// Request/response ID.
        /// Request CID will match response CID.
        /// If event was not a response cid will be None.
        pub cid: Option<String>,
        pub match_presence_event: Option<MatchPresenceEvent>,
        pub match_data: Option<MatchData>,
        #[nserde(rename = "match")]
        pub new_match: Option<Match>,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct Presence {
        pub user_id: String,
        pub session_id: String,
        pub username: String,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct MatchPresenceEvent {
        pub match_id: String,
        #[nserde(default)]
        pub joins: Vec<Presence>,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct MatchData {
        pub match_id: String,
        pub presence: Presence,
        #[nserde(default)]
        #[nserde(proxy = "Base64Encoder")]
        pub data: Vec<u8>,
        pub op_code: String,
        pub reliable: bool,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct Match {
        pub match_id: String,
        pub authoritative: bool,
        pub label: String,
        #[nserde(rename = "self")]
        pub self_user: Presence,
        #[nserde(default)]
        pub presences: Vec<Presence>,
    }

    #[derive(DeJson, Clone, Debug)]
    #[nserde(transparent)]
    struct Base64Encoder(String);
    impl From<&Base64Encoder> for Vec<u8> {
        fn from(base64: &Base64Encoder) -> Vec<u8> {
            let mut buffer = Vec::<u8>::new();
            base64::decode_config_buf(&base64.0, base64::STANDARD, &mut buffer).unwrap();
            buffer
        }
    }

    pub struct Socket {
        web_socket: WebSocket,
    }

    impl Socket {
        pub fn connect(addr: &str, port: u32, appear_online: bool, token: &str) -> Socket {
            let ws_addr = format!(
                "{}:{}/ws?lang=en&status={}&token={}",
                addr, port, appear_online, token
            );

            Socket {
                web_socket: WebSocket::connect(&ws_addr).unwrap(),
            }
        }

        pub fn connected(&self) -> bool {
            self.web_socket.connected()
        }

        pub fn try_recv(&mut self) -> Option<String> {
            self.web_socket
                .try_recv()
                .map(|bytes| String::from_utf8(bytes).unwrap())
        }

        pub fn join_match(&mut self, match_id: &str) {
            self.web_socket.send_text(&format!(
                r#"{{"match_join":{{"match_id":"{}"}},"cid":"1"}}"#,
                match_id
            ));
        }

        pub fn match_data_send(&mut self, match_id: &str, opcode: i32, data: &[u8]) {
            let mut buf = String::new();
            base64::encode_config_buf(data, base64::STANDARD, &mut buf);

            self.web_socket
                .send_text(&format!(
                    r#"{{"match_data_send":{{"match_id":"{}","op_code":"{}","data":"{}","presences":[]}}}}"#,
                    match_id, opcode, buf
                ));
        }
    }
}

pub mod async_client {
    use super::api;
    use quad_net::http_request::{Method, Request, RequestBuilder};

    pub struct AsyncRequest<T: nanoserde::DeJson> {
        _marker: std::marker::PhantomData<T>,
        request: Request,
    }

    impl<T: nanoserde::DeJson> AsyncRequest<T> {
        pub fn try_recv(&mut self) -> Option<T> {
            if let Some(response) = self.request.try_recv() {
                return Some(nanoserde::DeJson::deserialize_json(&response.unwrap()).unwrap());
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
}
