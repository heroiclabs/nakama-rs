-- Copyright 2021 The Nakama Authors
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
-- http:--www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

local nk = require("nakama")

local id = "example-tournament"
local sort = "desc"     -- one of: "desc", "asc"
local operator = "best" -- one of: "best", "set", "incr"
local reset = "0 12 * * *" -- noon UTC each day
local metadata = {
  weather_conditions = "rain"
}
local title = "Daily Dash"
local description = "Dash past your opponents for high scores and big rewards!"
local category = 1
local start_time = nk.time() / 1000 -- starts now in seconds
local end_time = 0                  -- never end, repeat the tournament each day forever
local duration = 36000               -- in seconds
local max_size = 10000              -- first 10,000 players who join
local max_num_score = 100             -- each player can have 3 attempts to score
local join_required = true          -- must join to compete
nk.tournament_create(id, sort, operator, duration, reset, metadata, title, description, category,
    start_time, end_time, max_size, max_num_score, join_required)

local id = "example-tournament2"
local sort = "desc"     -- one of: "desc", "asc"
local operator = "best" -- one of: "best", "set", "incr"
local reset = "0 12 * * *" -- noon UTC each day
local metadata = {
  weather_conditions = "rain"
}
local title = "Daily Dash 2"
local description = "Dash past your opponents for high scores and big rewards!"
local category = 1
local start_time = nk.time() / 1000 -- starts now in seconds
local end_time = 0                  -- never end, repeat the tournament each day forever
local duration = 36000               -- in seconds
local max_size = 10000              -- first 10,000 players who join
local max_num_score = 100             -- each player can have 3 attempts to score
local join_required = true          -- must join to compete
nk.tournament_create(id, sort, operator, duration, reset, metadata, title, description, category,
    start_time, end_time, max_size, max_num_score, join_required)
