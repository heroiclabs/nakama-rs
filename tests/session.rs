// Copyright 2021 The Nakama Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use std::collections::HashMap;

#[test]
fn test_session_variables() {
    let client = DefaultClient::new_with_adapter();

    let result = block_on(async {
        let mut vars = HashMap::new();
        vars.insert("ident".to_owned(), "hidden".to_owned());
        let mut session = client
            .authenticate_device("somenewdeviceid", None, true, vars)
            .await?;

        client.get_account(&mut session).await
    });

    println!("Result: {:?}", result);
    // TODO: parse "vrs" from the token payload
    // let account = result.unwrap();
}
