use query::response::Response;
use query::scope::{Scope, CompiledScope};
use query::threshold::Threshold;
use query::trigger::{Trigger, CompiledTrigger};

use common::compilation::CompilableTo;
use common::validation::{Issue, Validatable};

pub struct Query {
    response: Response,
    scope: Scope,
    threshold: Threshold,
    triggers: Vec<Trigger>,
    id: Option<String>,
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