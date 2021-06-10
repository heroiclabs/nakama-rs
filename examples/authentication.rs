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

fn main() {
    block_on(async {
        let client = DefaultClient::new_with_adapter_and_defaults();

        let result = client
            .authenticate_device("too_short", None, true, HashMap::new())
            .await;
        println!("{:?}", result);

        let result = client
            .authenticate_device("long_enough_device_id", None, true, HashMap::new())
            .await;

        println!("{:?}", result);
    })
}
