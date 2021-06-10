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

use std::error::Error;

pub trait SocketAdapter {
    type Error: Error;
    fn on_connected<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static;
    fn on_closed<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static;

    // TODO: correct error type
    fn on_received<T>(&mut self, callback: T)
    where
        T: Fn(Result<String, Self::Error>) + Send + 'static;

    fn is_connected(&self) -> bool;
    fn is_connecting(&self) -> bool;

    fn close(&mut self);

    fn connect(&mut self, addr: &str, timeout: i32);

    fn send(&self, data: &str, reliable: bool) -> Result<(), Self::Error>;

    fn tick(&self);
}
