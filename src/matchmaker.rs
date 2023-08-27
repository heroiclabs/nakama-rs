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

use std::collections::HashMap;

#[derive(Eq, PartialEq)]
enum RangeOperator {
    GT,
    LT,
    GEQ,
    LEQ,
}

enum QueryType {
    Term(String),
    Range { value: i32, operator: RangeOperator },
}

#[derive(Eq, PartialEq)]
enum Boolean {
    Required,
    Optional,
    Excluded,
}

pub struct Matchmaker {
    pub min_count: i32,
    pub max_count: i32,
    pub string_properties: HashMap<String, String>,
    pub numeric_properties: HashMap<String, f64>,
    pub query: String,
}

pub struct QueryItemBuilder {
    property: String,
    query_type: Option<QueryType>,
    boolean: Boolean,
    boost: i64,
}

impl QueryItemBuilder {
    pub fn new(property: &str) -> QueryItemBuilder {
        QueryItemBuilder {
            property: property.to_owned(),
            query_type: None,
            boolean: Boolean::Optional,
            boost: 0,
        }
    }
    pub fn term(&mut self, term: &str) -> &mut Self {
        self.query_type = Some(QueryType::Term(term.to_owned()));
        self
    }

    pub fn gt(&mut self, value: i32) -> &mut Self {
        self.query_type = Some(QueryType::Range {
            operator: RangeOperator::GT,
            value,
        });
        self
    }

    pub fn geq(&mut self, value: i32) -> &mut Self {
        self.query_type = Some(QueryType::Range {
            operator: RangeOperator::GEQ,
            value,
        });
        self
    }

    pub fn lt(&mut self, value: i32) -> &mut Self {
        self.query_type = Some(QueryType::Range {
            operator: RangeOperator::LT,
            value,
        });
        self
    }

    pub fn leq(&mut self, value: i32) -> &mut Self {
        self.query_type = Some(QueryType::Range {
            operator: RangeOperator::LEQ,
            value,
        });
        self
    }

    pub fn required(&mut self) -> &mut Self {
        self.boolean = Boolean::Required;
        self
    }

    pub fn excluded(&mut self) -> &mut Self {
        self.boolean = Boolean::Excluded;
        self
    }

    pub fn boost(&mut self, amount: i64) -> &mut Self {
        self.boost = amount;
        self
    }

    pub fn build(&mut self) -> String {
        assert!(self.query_type.is_some());

        let boolean = if self.boolean == Boolean::Required {
            "+"
        } else if self.boolean == Boolean::Excluded {
            "-"
        } else {
            ""
        };

        let field = format!("properties.{}:", self.property);

        let boost = if self.boost == 0 {
            "".to_owned()
        } else {
            format!("^{}", self.boost)
        };

        let ref query_type = self.query_type.as_ref().unwrap();

        match query_type {
            QueryType::Term(term) => format!("{}{}{}{}", boolean, field, term, boost),
            QueryType::Range { operator, value } => {
                let op = match operator {
                    RangeOperator::GT => ">",
                    RangeOperator::LT => "<",
                    RangeOperator::GEQ => ">=",
                    RangeOperator::LEQ => "<=",
                };
                format!("{}{}{}{}{}", boolean, field, op, value, boost)
            }
        }
    }
}

impl<'a> Matchmaker {
    pub fn new() -> Self {
        Matchmaker {
            min_count: 2,
            max_count: 100,
            string_properties: HashMap::new(),
            numeric_properties: HashMap::new(),
            query: "".to_owned(),
        }
    }

    pub fn string_properties(&self) -> String {
        let mut str = "{".to_owned();

        let properties = self
            .string_properties
            .iter()
            .map(|property| format!("\"{}\": \"{}\"", property.0, property.1))
            .collect::<Vec<String>>();

        str += &properties.join(",");

        str += "}";
        str
    }

    pub fn numeric_properties(&self) -> String {
        let mut str = "{".to_owned();

        let properties = self
            .numeric_properties
            .iter()
            .map(|property| format!("\"{}\": {}", property.0, property.1))
            .collect::<Vec<String>>();

        str += &properties.join(",");

        str += "}";
        str
    }

    pub fn property_exists(&self, name: &str) -> bool {
        self.string_properties.contains_key(name) || self.numeric_properties.contains_key(name)
    }

    pub fn min(&mut self, min: i32) -> &mut Self {
        self.min_count = min;
        self
    }

    pub fn max(&mut self, max: i32) -> &mut Self {
        self.max_count = max;
        self
    }

    pub fn add_string_property(&mut self, name: &str, value: &str) -> &mut Self {
        assert!(!self.property_exists(name));

        self.string_properties
            .insert(name.to_owned(), value.to_owned());

        self
    }

    pub fn add_numeric_property(&mut self, name: &str, value: f64) -> &mut Self {
        assert!(!self.property_exists(name));

        self.numeric_properties.insert(name.to_owned(), value);

        self
    }

    pub fn add_query_item(&mut self, query: &str) -> &mut Self {
        if self.query.len() != 0 {
            self.query.push(' ')
        }

        self.query.push_str(query);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_formatting() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_string_property("region", "Europe")
            .add_string_property("host", "Windows")
            .add_numeric_property("rank", 5.5)
            .add_numeric_property("elo", 1000.0);

        assert_eq!(
            matchmaker.string_properties(),
            "{\"host\": \"Windows\",\"region\": \"Europe\"}"
        );
        assert_eq!(
            matchmaker.numeric_properties(),
            "{\"elo\": 1000,\"rank\": 5.5}"
        );
    }

    #[test]
    fn term() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item(&QueryItemBuilder::new("region").term("europe").build());

        assert_eq!(matchmaker.query, "properties.region:europe");
    }

    #[test]
    fn range_lt() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item(&QueryItemBuilder::new("rank").lt(2).build());

        assert_eq!(matchmaker.query, "properties.rank:<2");
    }

    #[test]
    fn range_leq() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item(&QueryItemBuilder::new("rank").leq(2).build());

        assert_eq!(matchmaker.query, "properties.rank:<=2");
    }

    #[test]
    fn range_gt() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item(&QueryItemBuilder::new("rank").gt(2).build());

        assert_eq!(matchmaker.query, "properties.rank:>2");
    }

    #[test]
    fn range_geq() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item(&QueryItemBuilder::new("rank").geq(2).build());

        assert_eq!(matchmaker.query, "properties.rank:>=2");
    }

    #[test]
    fn boost() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item(&QueryItemBuilder::new("rank").geq(2).boost(5).build());

        assert_eq!(matchmaker.query, "properties.rank:>=2^5");
    }

    #[test]
    fn required() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item(
                &QueryItemBuilder::new("region")
                    .term("europe")
                    .required()
                    .build(),
            );

        assert_eq!(matchmaker.query, "+properties.region:europe");
    }

    #[test]
    fn excluded() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item(
                &QueryItemBuilder::new("region")
                    .term("europe")
                    .excluded()
                    .build(),
            );

        assert_eq!(matchmaker.query, "-properties.region:europe");
    }

    #[test]
    fn multiple_terms() {
        let mut matchmaker = Matchmaker::new();
        matchmaker
            .add_string_property("region", "Europe")
            .add_string_property("country", "Germany")
            .add_query_item(
                &QueryItemBuilder::new("region")
                    .term("europe")
                    .required()
                    .build(),
            )
            .add_query_item(
                &QueryItemBuilder::new("country")
                    .term("Germany")
                    .excluded()
                    .build(),
            );

        assert_eq!(
            matchmaker.query,
            "+properties.region:europe -properties.country:Germany"
        );
    }
}
