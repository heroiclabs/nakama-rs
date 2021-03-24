mod api_gen;

pub mod api {
    pub use super::api_gen::*;
}

pub mod rt_api {
    use nanoserde::DeJson;
    use std::sync::mpsc::{self, channel};

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

    struct Client {
        out: ws::Sender,
        thread_out: mpsc::Sender<Event>,
    }

    enum Event {
        Connect(ws::Sender),
        Message(String),
    }

    pub struct Socket {
        sender: ws::Sender,
        rx: mpsc::Receiver<Event>,
    }

    impl ws::Handler for Client {
        fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
            self.thread_out
                .send(Event::Connect(self.out.clone()))
                .unwrap();
            Ok(())
        }

        fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
            self.thread_out
                .send(Event::Message(msg.into_text().unwrap()))
                .unwrap();
            Ok(())
        }

        fn on_close(&mut self, code: ws::CloseCode, _reason: &str) {
            println!("closed {:?}", code);
        }

        fn on_error(&mut self, error: ws::Error) {
            println!("{:?}", error);
        }
    }

    impl Socket {
        pub fn connect(addr: &str, port: u32, appear_online: bool, token: &str) -> Socket {
            let ws_addr = format!(
                "{}:{}/ws?lang=en&status={}&token={}",
                addr, port, appear_online, token
            );

            let (tx, rx) = channel();
            std::thread::spawn(move || {
                ws::connect(ws_addr, |out| Client {
                    out,
                    thread_out: tx.clone(),
                })
                .unwrap()
            });

            match rx.recv() {
                Ok(Event::Connect(sender)) => Socket { sender, rx },
                _ => panic!("Failed to connect websocket"),
            }
        }

        pub fn try_recv(&mut self) -> Option<String> {
            self.rx.try_recv().ok().map(|event| match event {
                Event::Message(msg) => msg,
                _ => panic!(),
            })
        }

        pub fn join_match(&self, match_id: &str) {
            self.sender
                .send(format!(
                    r#"{{"match_join":{{"match_id":"{}"}},"cid":"1"}}"#,
                    match_id
                ))
                .unwrap();
        }

        pub fn match_data_send(&self, match_id: &str, opcode: i32, data: &[u8]) {
            let mut buf = String::new();
            base64::encode_config_buf(data, base64::STANDARD, &mut buf);

            self.sender
                .send(format!(
                    r#"{{"match_data_send":{{"match_id":"{}","op_code":"{}","data":"{}","presences":[]}}}}"#,
                    match_id, opcode, buf
                ))
                .unwrap();
        }
    }
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

        let response = make_request("http://127.0.0.1", 7350, request);
        println!("{:?}", response);

        println!("connecting socket");
        let nakama_socket =
            crate::rt_api::Socket::connect("ws://127.0.0.1", 7350, false, &response.token);
        nakama_socket.join_match("asd");
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
