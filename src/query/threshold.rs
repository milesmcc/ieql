use std::collections::HashMap;
use common::validation::Issue;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Threshold {
    pub considers: Vec<ThresholdConsideration>, // todo: make this a reference
    pub requires: usize,
    pub inverse: bool
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ThresholdConsideration {
    Trigger(String),
    NestedThreshold(Threshold),
}

impl Threshold {
    fn evaluate(&self, triggers: &HashMap<String, bool>) -> Result<bool, Issue> {
        let mut matched = 0;
        
        for consideration in &self.considers {
            if match consideration {
                ThresholdConsideration::Trigger(id) => match triggers.get(id) {
                    Some(res) => *res,
                    None => return Err(Issue::Error(format!("unable to find trigger `{}` in given triggers", id)))
                },
                ThresholdConsideration::NestedThreshold(threshold) => match threshold.evaluate(triggers) {
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