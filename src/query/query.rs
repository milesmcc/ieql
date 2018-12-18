use query::response::Response;
use query::scope::{Scope, CompiledScope};
use query::threshold::Threshold;
use query::trigger::{Trigger, CompiledTrigger};

use common::compilation::CompilableTo;
use common::validation::{Issue, Validatable};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
        }
    }

    #[test]
    fn test_basic_serialization(){
        let serialized_object_ron = ron::ser::to_string(&get_basic_query()).unwrap();
        assert_eq!(serialized_object_ron, "(response:(kind:Full,include:[Excerpt,Url,],),scope:(pattern:(content:\".+\",kind:RegEx,),content:Raw,),threshold:(considers:[Trigger(\"A\"),NestedThreshold((considers:[Trigger(\"B\"),Trigger(\"C\"),],requires:1,inverse:false,)),],requires:2,inverse:false,),triggers:[(pattern:(content:\"hello\",kind:RegEx,),id:\"A\",),(pattern:(content:\"everyone\",kind:RegEx,),id:\"B\",),(pattern:(content:\"around\",kind:RegEx,),id:\"C\",),],id:Some(\"Test Trigger #1\"),)")
    }

    #[test]
    fn test_basic_deserialization(){
        let basic_query: Query = ron::de::from_str("(response:(kind:Full,include:[Excerpt,Url,],),scope:(pattern:(content:\".+\",kind:RegEx,),content:Raw,),threshold:(considers:[Trigger(\"A\"),NestedThreshold((considers:[Trigger(\"B\"),Trigger(\"C\"),],requires:1,inverse:false,)),],requires:2,inverse:false,),triggers:[(pattern:(content:\"hello\",kind:RegEx,),id:\"A\",),(pattern:(content:\"everyone\",kind:RegEx,),id:\"B\",),(pattern:(content:\"around\",kind:RegEx,),id:\"C\",),],id:Some(\"Test Trigger #1\"),)").unwrap();
        assert_eq!(get_basic_query(), basic_query);
    }
}