use query::response::Response;
use query::scope::{CompiledScope, Scope, ScopeContent};
use query::threshold::{Threshold, ThresholdConsideration};
use query::trigger::{CompiledTrigger, Trigger};

use common::compilation::CompilableTo;
use common::validation::{Issue, Validatable};

use regex::{Regex, RegexSet};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Query {
    pub response: Response,
    pub scope: Scope,
    pub threshold: Threshold,
    pub triggers: Vec<Trigger>,
    pub id: Option<String>,
}

pub struct QueryGroup {
    pub queries: Vec<Query>,
}

#[derive(Clone)]
pub struct CompiledQuery {
    pub response: Response,
    pub scope: CompiledScope,
    pub threshold: Threshold,
    pub triggers: Vec<CompiledTrigger>,
    pub id: Option<String>,
}

#[derive(Clone)]
pub struct CompiledQueryGroup {
    pub queries: Vec<CompiledQuery>,
    pub regex_collected: RegexSet,
    pub regex_collected_query_index: Vec<usize>,
    pub always_run_queries: Vec<CompiledQuery>, // for unoptimizable queries
    pub regex_feed: ScopeContent
}

impl CompilableTo<CompiledQuery> for Query {
    fn compile(&self) -> Result<CompiledQuery, Issue> {
        let scope = match self.scope.compile() {
            Ok(compiled) => compiled,
            Err(issue) => return Err(issue),
        };

        let mut triggers: Vec<CompiledTrigger> = Vec::new();
        for trigger in &self.triggers {
            let compiled_trigger = match trigger.compile() {
                Ok(compiled) => compiled,
                Err(issue) => return Err(issue),
            };
            triggers.push(compiled_trigger)
        }

        Ok(CompiledQuery {
            response: self.response.clone(),
            scope: scope,
            threshold: self.threshold.clone(),
            triggers: triggers,
            id: self.id.clone(),
        })
    }
}

impl CompilableTo<CompiledQueryGroup> for QueryGroup {
    fn compile(&self) -> Result<CompiledQueryGroup, Issue> {
        let OPTIMIZE_FOR_SCOPE_CONTENT = ScopeContent::Raw;

        let mut queries: Vec<CompiledQuery> = Vec::new();
        let mut sub_regexes: Vec<String> = Vec::new();
        let mut sub_regexes_index: Vec<usize> = Vec::new();
        let mut always_runs: Vec<CompiledQuery> = Vec::new();

        // Returns a tuple of the 0) relevant trigger IDs and 2) whether the query is an always-run
        fn recursively_analyze_threshold(threshold: &Threshold) -> (Vec<&String>, bool) {
            let mut relevant_triggers: Vec<&String> = Vec::new();
            let mut is_always = false;

            for consideration in &threshold.considers {
                match consideration {
                    ThresholdConsideration::NestedThreshold(nested_threshold) => {
                        let (nested_triggers, nested_always) = recursively_analyze_threshold(&nested_threshold);
                        if nested_always {
                            is_always = true;
                        }
                        relevant_triggers.extend(nested_triggers);
                    }
                    ThresholdConsideration::Trigger(id) => relevant_triggers.push(id), // cloning is OK
                }
            }

            if threshold.inverse {
                is_always = true;
            }

            (relevant_triggers, is_always)
        }

        for query in &self.queries {
            let compiled_query = match query.compile() {
                Ok(compiled_query) => compiled_query,
                Err(issue) => return Err(issue), // kill early; compilation is expensive!
            };
            let (relevant_trigger_ids, is_inverse) = recursively_analyze_threshold(&query.threshold);
            if is_inverse || (query.scope.content != OPTIMIZE_FOR_SCOPE_CONTENT) {
                always_runs.push(compiled_query);
            } else {
                let query_index = queries.len();
                for trigger in &query.triggers {
                    if relevant_trigger_ids.contains(&&trigger.id) {
                        let regex_smart = trigger.pattern.get_as_safe_regex();
                        sub_regexes.push(regex_smart);
                        sub_regexes_index.push(query_index);
                    }
                }
                queries.push(compiled_query);
            }
        }

        let regex_set = match RegexSet::new(sub_regexes) {
            Ok(set) => set,
            Err(_iss) => return Err(Issue::Error(String::from("unable to compile master regex set")))
        };

        Ok(CompiledQueryGroup {
            queries: queries,
            regex_collected: regex_set,
            regex_collected_query_index: sub_regexes_index,
            always_run_queries: always_runs,
            regex_feed: OPTIMIZE_FOR_SCOPE_CONTENT
        })
    }
}

impl From<CompiledQuery> for CompiledQueryGroup { // CompiledQueryGroup can represent a single query
    fn from(query: CompiledQuery) -> CompiledQueryGroup {
        CompiledQueryGroup { // it will always be faster to just run the query than to perform optimizations designed with multiple queries in mind
            queries: vec![],
            regex_collected: RegexSet::new(vec![""]).unwrap(),
            regex_collected_query_index: vec![],
            always_run_queries: vec![query], // for unoptimizable queries
            regex_feed: ScopeContent::Raw,
        }
    }
}

impl Validatable for Query {
    fn validate(&self) -> Option<Vec<Issue>> {
        let mut issues: Vec<Issue> = Vec::new();

        // Check if it compiles
        match self.compile() {
            Ok(_) => (),
            Err(issue) => {
                issues.push(issue);
            }
        }

        // Check response validity
        match self.response.validate() {
            Some(problems) => issues.extend(problems),
            None => (),
        }

        // Check threshold validity
        let mut trigger_responses: HashMap<&String, bool> = HashMap::new();
        for trigger in &self.triggers {
            trigger_responses.insert(&trigger.id, false);
        }
        match self.threshold.evaluate(&trigger_responses) {
            Ok(value) => {
                if value == true {
                    issues.push(Issue::Warning(String::from("query will match if all triggers do not match; this can be dangerous in certain situations")));
                }
            },
            Err(issue) => issues.push(issue),
        };
        if issues.len() > 0 {
            return Some(issues);
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::pattern::*;
    use query::response::*;
    use query::scope::*;
    use query::threshold::*;
    use query::trigger::*;

    use ron;

    fn get_basic_query() -> Query {
        Query {
            response: Response {
                kind: ResponseKind::Full,
                include: vec![ResponseItem::Excerpt, ResponseItem::Url],
            },
            scope: Scope {
                pattern: Pattern {
                    content: String::from(".+"),
                    kind: PatternKind::RegEx,
                },
                content: ScopeContent::Raw,
            },
            threshold: Threshold {
                considers: vec![
                    ThresholdConsideration::Trigger(String::from("A")),
                    ThresholdConsideration::NestedThreshold(Threshold {
                        considers: vec![
                            ThresholdConsideration::Trigger(String::from("B")),
                            ThresholdConsideration::Trigger(String::from("C")),
                        ],
                        inverse: false,
                        requires: 1,
                    }),
                ],
                inverse: false,
                requires: 2,
            },
            triggers: vec![
                Trigger {
                    pattern: Pattern {
                        content: String::from("hello"),
                        kind: PatternKind::RegEx,
                    },
                    id: String::from("A"),
                },
                Trigger {
                    pattern: Pattern {
                        content: String::from("everyone"),
                        kind: PatternKind::RegEx,
                    },
                    id: String::from("B"),
                },
                Trigger {
                    pattern: Pattern {
                        content: String::from("around"),
                        kind: PatternKind::RegEx,
                    },
                    id: String::from("C"),
                },
            ],
            id: Some(String::from("Test Trigger #1")),
        }
    }

    #[test]
    fn test_basic_serialization() {
        let serialized_object_ron = ron::ser::to_string(&get_basic_query()).unwrap();
        assert_eq!(serialized_object_ron, "(response:(kind:Full,include:[Excerpt,Url,],),scope:(pattern:(content:\".+\",kind:RegEx,),content:Raw,),threshold:(considers:[Trigger(\"A\"),NestedThreshold((considers:[Trigger(\"B\"),Trigger(\"C\"),],requires:1,inverse:false,)),],requires:2,inverse:false,),triggers:[(pattern:(content:\"hello\",kind:RegEx,),id:\"A\",),(pattern:(content:\"everyone\",kind:RegEx,),id:\"B\",),(pattern:(content:\"around\",kind:RegEx,),id:\"C\",),],id:Some(\"Test Trigger #1\"),)")
    }

    #[test]
    fn test_basic_deserialization() {
        let basic_query: Query = ron::de::from_str("(response:(kind:Full,include:[Excerpt,Url,],),scope:(pattern:(content:\".+\",kind:RegEx,),content:Raw,),threshold:(considers:[Trigger(\"A\"),NestedThreshold((considers:[Trigger(\"B\"),Trigger(\"C\"),],requires:1,inverse:false,)),],requires:2,inverse:false,),triggers:[(pattern:(content:\"hello\",kind:RegEx,),id:\"A\",),(pattern:(content:\"everyone\",kind:RegEx,),id:\"B\",),(pattern:(content:\"around\",kind:RegEx,),id:\"C\",),],id:Some(\"Test Trigger #1\"),)").unwrap();
        assert_eq!(get_basic_query(), basic_query);
    }

    #[test]
    fn test_basic_compilation() {
        let basic_query = get_basic_query();
        basic_query.compile(); // can it do it without panicking?
    }

    #[test]
    fn test_group_compilation() {
        let mut queries: Vec<Query> = Vec::new();
        for _i in 0..20 {
            queries.push(get_basic_query());
        }
        let _special_query = Query {
            response: Response {
                kind: ResponseKind::Full,
                include: vec![ResponseItem::Excerpt, ResponseItem::Url],
            },
            scope: Scope {
                pattern: Pattern {
                    content: String::from(".+"),
                    kind: PatternKind::RegEx,
                },
                content: ScopeContent::Raw,
            },
            threshold: Threshold {
                considers: vec![
                    ThresholdConsideration::Trigger(String::from("A")),
                    ThresholdConsideration::NestedThreshold(Threshold {
                        considers: vec![
                            ThresholdConsideration::Trigger(String::from("B")),
                            ThresholdConsideration::Trigger(String::from("C")),
                        ],
                        inverse: true,
                        requires: 1,
                    }),
                ],
                inverse: false,
                requires: 2,
            },
            triggers: vec![
                Trigger {
                    pattern: Pattern {
                        content: String::from("hello"),
                        kind: PatternKind::RegEx,
                    },
                    id: String::from("A"),
                },
                Trigger {
                    pattern: Pattern {
                        content: String::from("everyone"),
                        kind: PatternKind::RegEx,
                    },
                    id: String::from("B"),
                },
                Trigger {
                    pattern: Pattern {
                        content: String::from("around"),
                        kind: PatternKind::RegEx,
                    },
                    id: String::from("C"),
                },
            ],
            id: Some(String::from("Test Trigger #2 (inverse)")),
        };
        let group = QueryGroup {
            queries: queries
        };
        group.compile();
    }
}
