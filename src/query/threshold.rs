use std::collections::HashMap;
use common::validation::Issue;

#[derive(Clone)]
pub struct Threshold {
    considers: Vec<Consideration>, // todo: make this a reference
    requires: usize,
    inverse: bool,
}

#[derive(Clone)]
pub enum Consideration {
    Trigger(String),
    NestedThreshold(Threshold),
}

impl Threshold {
    fn evaluate(&self, triggers: &HashMap<String, bool>) -> Result<bool, Issue> {
        let mut matched = 0;
        
        for consideration in &self.considers {
            if match consideration {
                Consideration::Trigger(id) => match triggers.get(id) {
                    Some(res) => *res,
                    None => return Err(Issue::Error(format!("unable to find trigger `{}` in given triggers", id)))
                },
                Consideration::NestedThreshold(threshold) => match threshold.evaluate(triggers) {
                    Ok(res) => res,
                    Err(issue) => return Err(issue)
                }
            } {
                matched += 1;
            }
        }

        let mut does_match = matched >= self.requires;

        if self.inverse {
            does_match = !does_match;
        }

        Ok(does_match)
    }
}