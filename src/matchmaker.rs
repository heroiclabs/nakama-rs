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
    Phrase(String),
    Range { value: i32, operator: RangeOperator },
}

#[derive(Eq, PartialEq)]
enum Boolean {
    Required,
    Optional,
    Excluded,
}

pub struct Matchmaker {
    pub min_count: u32,
    pub max_count: u32,
    string_properties: HashMap<String, String>,
    numeric_properties: HashMap<String, f64>,
    pub query: String,
}

pub struct QueryItemBuilder<'a> {
    matchmaker: &'a mut Matchmaker,
    property: &'a str,
    query_type: Option<QueryType>,
    boolean: Boolean,
    boost: i64,
}

impl<'a> QueryItemBuilder<'a> {
    pub fn term(&mut self, term: &str) -> &mut Self {
        self.query_type = Some(QueryType::Term(term.to_owned()));
        self
    }

    pub fn phrase(&mut self, phrase: &str) -> &mut Self {
        self.query_type = Some(QueryType::Phrase(phrase.to_owned()));
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

    pub fn build(&'a mut self) -> &'a mut Matchmaker {
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

        if self.matchmaker.query.len() != 0 {
            self.matchmaker.query.push(' ')
        }

        match query_type {
            QueryType::Term(term) => self
                .matchmaker
                .query
                .push_str(format!("{}{}{}{}", boolean, field, term, boost).as_str()),
            QueryType::Phrase(phrase) => self
                .matchmaker
                .query
                .push_str(format!("{}{}\"{}\"{}", boolean, field, phrase, boost).as_str()),
            QueryType::Range { operator, value } => {
                let op = match operator {
                    RangeOperator::GT => ">",
                    RangeOperator::LT => "<",
                    RangeOperator::GEQ => ">=",
                    RangeOperator::LEQ => "<=",
                };
                self.matchmaker
                    .query
                    .push_str(format!("{}{}{}{}{}", boolean, field, op, value, boost).as_str())
            }
        }

        self.matchmaker
    }
}

impl<'a> Matchmaker {
    pub fn new(min_count: u32, max_count: u32) -> Self {
        Matchmaker {
            min_count,
            max_count,
            string_properties: HashMap::new(),
            numeric_properties: HashMap::new(),
            query: "".to_owned(),
        }
    }

    pub fn string_properties(&self) -> String {
        let mut str = "{".to_owned();
        for property in &self.string_properties {
           str += format!("\"{}\": \"{}\"", property.0, property.1).as_str();
        }
        str += "}";
        str
    }

    pub fn numeric_properties(&self) -> String {
        let mut str = "{".to_owned();
        for property in &self.numeric_properties {
            str += format!("\"{}\": \"{}\"", property.0, property.1).as_str();
        }
        str += "}";
        str
    }

    pub fn property_exists(&self, name: &str) -> bool {
        self.string_properties.contains_key(name) || self.numeric_properties.contains_key(name)
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

    pub fn add_query_item(&'a mut self, property: &'a str) -> QueryItemBuilder<'a> {
        QueryItemBuilder {
            matchmaker: self,
            property,
            boolean: Boolean::Optional,
            query_type: None,
            boost: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item("region")
            .term("europe")
            .build();

        assert_eq!(matchmaker.query, "properties.region:europe");
    }

    #[test]
    fn phrase() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item("region")
            .phrase("europe")
            .build();

        assert_eq!(matchmaker.query, "properties.region:\"europe\"");
    }

    #[test]
    fn range_lt() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item("rank")
            .lt(2)
            .build();

        assert_eq!(matchmaker.query, "properties.rank:<2");
    }

    #[test]
    fn range_leq() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item("rank")
            .leq(2)
            .build();

        assert_eq!(matchmaker.query, "properties.rank:<=2");
    }

    #[test]
    fn range_gt() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item("rank")
            .gt(2)
            .build();

        assert_eq!(matchmaker.query, "properties.rank:>2");
    }

    #[test]
    fn range_geq() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item("rank")
            .geq(2)
            .build();

        assert_eq!(matchmaker.query, "properties.rank:>=2");
    }

    #[test]
    fn boost() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_numeric_property("rank", 5.0)
            .add_query_item("rank")
            .geq(2)
            .boost(5)
            .build();

        assert_eq!(matchmaker.query, "properties.rank:>=2^5");
    }

    #[test]
    fn required() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item("region")
            .term("europe")
            .required()
            .build();

        assert_eq!(matchmaker.query, "+properties.region:europe");
    }

    #[test]
    fn excluded() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_string_property("region", "Europe")
            .add_query_item("region")
            .term("europe")
            .excluded()
            .build();

        assert_eq!(matchmaker.query, "-properties.region:europe");
    }

    #[test]
    fn multiple_terms() {
        let mut matchmaker = Matchmaker::new(2, 4);
        matchmaker
            .add_string_property("region", "Europe")
            .add_string_property("country", "Germany")
            .add_query_item("region")
            .term("europe")
            .required()
            .build()
            .add_query_item("country")
            .term("Germany")
            .excluded()
            .build();

        assert_eq!(matchmaker.query, "+properties.region:europe -properties.country:Germany");
    }
}
