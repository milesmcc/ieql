//! This file contains functionality related to queries.

use query::response::Response;
use query::scope::{CompiledScope, Scope, ScopeContent};
use query::threshold::{Threshold, ThresholdConsideration};
use query::trigger::{CompiledTrigger, Trigger};

use common::compilation::CompilableTo;
use common::validation::{Issue, Validatable};

use regex::RegexSet;

use std::collections::HashMap;

/// `Query` represents an uncompiled query. This type is
/// typically interstitial; it cannot perform scans, and has
/// little functionality apart from its ability to compile into
/// a `CompiledQuery`.
/// 
/// This type is part of the public API, and therefore must
/// comply with the structure defined in the specification.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Query {
    /// Represents the desired `Response` of the query when
    /// it matches `Document`s. In other words, this is the
    /// set of parameters for the `Output`.
    /// 
    /// For more information, see the `Response` documentation.
    pub response: Response,
    /// Represents the documents and content that the query
    /// is applicable to. The `Scope` allows users to specify
    /// which URLs their query is applicable to, as well as
    /// the type of content—`Text` or `Raw`—that should be passed
    /// to the triggers.
    /// 
    /// For more information, see the `Scope` documentation.
    pub scope: Scope,
    /// Represents the composition of trigger matches necessary
    /// in order for a match to be made.
    /// 
    /// For more information, see the `Threshold` documentation.
    pub threshold: Threshold,
    /// Represents the `Triggers` that will be checked against
    /// the document and then processed by the `Threshold` in
    /// order to determine whether a match is made.
    /// 
    /// For more information, see the `Trigger` documentation.
    pub triggers: Vec<Trigger>,
    /// Represents the `id` of the query. This field is optional
    /// but highly recommended, as it will be copied to the
    /// outputs created by this query.
    pub id: Option<String>,
}

/// Represents a collection of queries. This type is useful in
/// cases where many different queries are being compiled at
/// once.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct QueryGroup {
    pub queries: Vec<Query>,
}

/// Represents a compiled query which is ready to scan (compiled)
/// documents (`CompiledDocument`).
/// 
/// For more information about each of these fields, please see the
/// `Query` documentation.
#[derive(Clone)]
pub struct CompiledQuery {
    pub response: Response,
    pub scope: CompiledScope,
    pub threshold: Threshold,
    pub triggers: Vec<CompiledTrigger>,
    pub id: Option<String>,
}

/// Represents a group of compiled queries. This type has several
/// internal optimizations that makes scanning using a
/// `CompiledQueryGroup` far more efficient than scanning using
/// compiled queries alone.
/// 
/// Namely, this type provides:
/// * proper multithreading support
/// * runtime optimizations for query execution
///
/// For information about how each of these optimizations
/// are implemented, see the documentation for
/// `scan_concurrently()` and the various fields.
/// 
/// There are few cases—if not none at all—when a `CompiledQueryGroup`
/// should be mutable.
#[derive(Clone)]
pub struct CompiledQueryGroup {
    /// Contains the compiled queries that make up the compiled
    /// query group.
    /// 
    /// Importantly, this vector is _highly ordered_,
    /// meaning that changing the order of this array will lead
    /// to the entire scanning system breaking.
    pub queries: Vec<CompiledQuery>,
    /// Contains every single RegEx pattern of every query's
    /// triggers.
    /// 
    /// This highly efficient RegEx matching system
    /// allows for the `CompiledQueryGroup`'s scanning mechanism
    /// to know in advance which of its queries will _potentially
    /// match_ on a document, without having to execute every
    /// individual query.
    pub regex_collected: RegexSet,
    /// This index relates every RegEx pattern in `regex_collected`
    /// to its source query in `queries`.
    /// 
    /// For example, the 1st element of this vector corresponds to the 
    /// 1st RegEx pattern in `regex_collected` and denotes the index
    /// of its source query in `queries`.
    pub regex_collected_query_index: Vec<usize>,
    /// Contains the queries that cannot be optimized using the
    /// methods above, and therefore must be run on every document.
    /// 
    /// These queries typically are those that contain some sort of
    /// inverse boolean operator, therefore making it possible for
    /// the query to match even when none of its triggers match.
    /// 
    /// Queries that have a threshold with a `requires` value of `0`
    /// and queries whose `ScopeContent` doesn't match the majority
    /// are also included here as unoptimizable.
    pub always_run_queries: Vec<CompiledQuery>,
    /// The type of content that should be fed to the RegEx patterns
    /// in `regex_collected`.
    pub regex_feed: ScopeContent,
}

impl CompilableTo<CompiledQuery> for Query {
    /// Compiles the `Query` into a `CompiledQuery`. Like all compilation
    /// operations, this is expensive.
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
    /// Compiles the `QueryGroup` into a `CompiledQueryGroup`. Like
    /// all compilation operations, this is expensive.
    fn compile(&self) -> Result<CompiledQueryGroup, Issue> {
        let optimized_content = ScopeContent::Text;

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
                        let (nested_triggers, nested_always) =
                            recursively_analyze_threshold(&nested_threshold);
                        if nested_always {
                            is_always = true;
                        }
                        relevant_triggers.extend(nested_triggers);
                    }
                    ThresholdConsideration::Trigger(id) => relevant_triggers.push(id), // cloning is OK
                }
            }

            if threshold.inverse || (threshold.requires == 0) {
                is_always = true;
            }

            (relevant_triggers, is_always)
        }

        for query in &self.queries {
            let compiled_query = match query.compile() {
                Ok(compiled_query) => compiled_query,
                Err(issue) => return Err(issue), // kill early; compilation is expensive!
            };
            let (relevant_trigger_ids, is_inverse) =
                recursively_analyze_threshold(&query.threshold);
            if is_inverse || (query.scope.content != optimized_content) {
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
            Err(_iss) => {
                return Err(Issue::Error(String::from(
                    "unable to compile master regex set",
                )))
            }
        };

        Ok(CompiledQueryGroup {
            queries: queries,
            regex_collected: regex_set,
            regex_collected_query_index: sub_regexes_index,
            always_run_queries: always_runs,
            regex_feed: optimized_content,
        })
    }
}

impl From<CompiledQuery> for CompiledQueryGroup {
    /// This helper function creates a `CompiledQueryGroup`
    /// for single queries, enabling multithreading support for
    /// single queries without any significant 'hacks.'
    fn from(query: CompiledQuery) -> CompiledQueryGroup {
        CompiledQueryGroup {
            // it will always be faster to just run the query
            // than to perform optimizations designed with
            // multiple queries in mind
            queries: vec![],
            regex_collected: RegexSet::new(vec![""]).unwrap(),
            regex_collected_query_index: vec![],
            always_run_queries: vec![query], // for unoptimizable queries
            regex_feed: ScopeContent::Raw,
        }
    }
}

impl Validatable for Query {
    /// Validates the query, as well as all of its sub-components.
    /// 
    /// While this is an important mechanism for proper validation
    /// (note that some invalid queries will still compile), one should
    /// always double-check the results by actually compiling the query
    /// and performing a test scan.
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
            }
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
        assert!(basic_query.compile().is_ok()); // can it do it without panicking?
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
        let group = QueryGroup { queries: queries };
        assert!(group.compile().is_ok());
    }
}
