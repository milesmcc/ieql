use query::response::Response;
use query::scope::{Scope, CompiledScope};
use query::threshold::Threshold;
use query::trigger::{Trigger, CompiledTrigger};

use common::compilation::CompilableTo;
use common::validation::{Issue, Validatable};

#[derive(Serialize, Deserialize)]
pub struct Query {
    pub response: Response,
    pub scope: Scope,
    pub threshold: Threshold,
    pub triggers: Vec<Trigger>,
    pub id: Option<String>,
}

pub struct CompiledQuery {
    response: Response,
    scope: CompiledScope,
    threshold: Threshold,
    triggers: Vec<CompiledTrigger>,
    id: Option<String>,
}

impl CompilableTo<CompiledQuery> for Query {
    fn compile(&self) -> Result<CompiledQuery, Issue> {
        let scope = match self.scope.compile() {
            Ok(compiled) => compiled,
            Err(issue) => return Err(issue)
        };
        
        let mut triggers: Vec<CompiledTrigger> = Vec::new();
        for trigger in &self.triggers {
            let compiled_trigger = match trigger.compile() {
                Ok(compiled) => compiled,
                Err(issue) => return Err(issue)
            };
            triggers.push(compiled_trigger)
        }
        
        Ok(CompiledQuery {
            response: self.response.clone(),
            scope: scope,
            threshold: self.threshold.clone(),
            triggers: triggers,
            id: self.id.clone()
        })
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
            None => ()
        }

        if issues.len() > 0 {
            return Some(issues);
        }else{
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use query::response::*;
    use query::scope::*;
    use query::threshold::*;
    use query::trigger::*;
    use common::pattern::*;

    use ron;

    #[test]
    fn check_basic_serialization(){
        let basic_query = Query {
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
                considers: vec![ThresholdConsideration::Trigger(String::from("A")), ThresholdConsideration::NestedThreshold(
                    Threshold {
                        considers: vec![ThresholdConsideration::Trigger(String::from("B")), ThresholdConsideration::Trigger(String::from("C"))],
                        inverse: false,
                        requires: 1,
                    }
                )],
                inverse: false,
                requires: 2,
            },
            triggers: vec![Trigger {
                pattern: Pattern {
                    content: String::from("hello"),
                    kind: PatternKind::RegEx,
                },
                id: String::from("A"),
            }, Trigger {
                pattern: Pattern {
                    content: String::from("everyone"),
                    kind: PatternKind::RegEx,
                },
                id: String::from("B"),
            }, Trigger {
                pattern: Pattern {
                    content: String::from("around"),
                    kind: PatternKind::RegEx,
                },
                id: String::from("C"),
            }],
            id: Some(String::from("Test Trigger #1")),
        };
        let serialized_object_ron = ron::ser::to_string_pretty(&basic_query, ron::ser::PrettyConfig::default()).unwrap();
        println!("RON -> {}", serialized_object_ron);
    }
}