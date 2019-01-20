//! This document provides functionality related to
//! thresholds.

use std::collections::HashMap;
use common::validation::Issue;

/// The `Threshold` struct allows for the boolean output of
/// triggers to be composed so that only certain combinations
/// constitute a 'match.'
/// 
/// You can think of the `Threshold` as a boolean expression
/// that defines when a query matches.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Threshold {
    pub considers: Vec<ThresholdConsideration>,
    pub requires: usize,
    pub inverse: bool
}

/// A consideration by the threshold that evaluates to
/// either `true` or `false`. This can be a `Trigger`
/// identified by its `id`, or a `NestedThreshold`.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ThresholdConsideration {
    /// Refers to a Trigger in the query by its ID.
    Trigger(String),
    /// Contains a Threshold which will then itself
    /// be independently evaluated by the scan engine.
    NestedThreshold(Threshold),
}

impl Threshold {
    /// Evaluates the threshold based on the given data.
    /// 
    /// # Arguments
    /// * triggers: a `HashMap` where the keys are Trigger IDs and the values are whether they matched or not
    pub fn evaluate(&self, triggers: &HashMap<&String, bool>) -> Result<bool, Issue> {
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