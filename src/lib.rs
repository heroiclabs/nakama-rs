mod api_gen;

pub mod api_client;

pub mod async_client;

pub mod api {
    pub use super::api_gen::*;
}

pub mod rt_api {
    use nanoserde::DeJson;
    use quad_net::web_socket::WebSocket;

    #[derive(DeJson, Debug, Clone)]
    pub struct SocketEvent {
        /// Request/response ID.
        /// Request CID will match response CID.
        /// If event was not a response cid will be None.
        pub cid: Option<String>,
        pub match_presence_event: Option<MatchPresenceEvent>,
        pub match_data: Option<MatchData>,
        #[nserde(rename = "match")]
        pub new_match: Option<Match>,
        pub matchmaker_matched: Option<MatchmakerMatched>,
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
        #[nserde(default)]
        pub leaves: Vec<Presence>,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct MatchData {
        pub match_id: String,
        pub presence: Presence,
        #[nserde(default)]
        #[nserde(proxy = "Base64Encoder")]
        pub data: Vec<u8>,
        pub op_code: String,
        #[nserde(default)]
        pub reliable: bool,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct Match {
        pub match_id: String,
        #[nserde(default)]
        pub authoritative: bool,
        #[nserde(default)]
        pub label: String,
        #[nserde(rename = "self")]
        pub self_user: Presence,
        #[nserde(default)]
        pub presences: Vec<Presence>,
    }

    #[derive(DeJson, Debug, Clone)]
    pub struct MatchmakerMatched {
        pub ticket: String,
        pub token: String,
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
        cid: u32,
    }

    impl Socket {
        pub fn connect(addr: &str, port: u32, appear_online: bool, token: &str) -> Socket {
            let ws_addr = format!(
                "{}:{}/ws?lang=en&status={}&token={}",
                addr, port, appear_online, token
            );

            Socket {
                web_socket: WebSocket::connect(&ws_addr).unwrap(),
                cid: 1,
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

        pub fn join_match_by_id(&mut self, match_id: &str) -> u32 {
            let id = self.cid;
            self.web_socket.send_text(&format!(
                r#"{{"match_join":{{"match_id":"{}"}},"cid":"{}"}}"#,
                match_id, id
            ));

            self.cid += 1;
            id
        }

        pub fn join_match_by_token(&mut self, token: &str) -> u32 {
            let id = self.cid;
            self.web_socket.send_text(&format!(
                r#"{{"match_join":{{"token":"{}"}},"cid":"{}"}}"#,
                token, id
            ));

            self.cid += 1;
            id
        }

        pub fn leave_match(&mut self, match_id: &str) -> u32 {
            let id = self.cid;
            self.web_socket.send_text(&format!(
                r#"{{"match_leave":{{"match_id":"{}"}},"cid":"{}"}}"#,
                match_id, id
            ));
            self.cid += 1;
            id
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

        /// usage example: `add_matchmaker(2, 4, "+properties.engine:\\\"macroquad_matchmaking\\\"", "{\"engine\":\"macroquad_matchmaking\"}");`
        pub fn add_matchmaker(
            &mut self,
            min_count: u32,
            max_count: u32,
            query: &str,
            string_properties: &str,
        ) -> u32 {
            let id = self.cid;
            let request = format!(
                r#"{{"matchmaker_add":{{"min_count":{},"max_count":{},"query":"{}","string_properties":{}}},"cid":"{}"}}"#,
                min_count, max_count, query, string_properties, id
            );

            self.web_socket.send_text(&request);
            self.cid += 1;
            id
        }

        pub fn create_match(&mut self) -> u32 {
            let id = self.cid;
            let request = format!(r#"{{"match_create":{{}},"cid":"{}"}}"#, id);
            self.web_socket.send_text(&request);
            self.cid += 1;
            id
        }
    }
}
