use input::document::DocumentBatch;
use output::output::{OutputBatch, Output, assemble};
use query::query::{CompiledQuery, CompiledQueryGroup};
use common::pattern::PatternMatch;
use std::collections::HashMap;

pub trait Scanner {
    fn scan(&self, documents: DocumentBatch) -> OutputBatch;
}

impl Scanner for CompiledQuery {
    fn scan(&self, documents: DocumentBatch) -> OutputBatch {
        let mut outputs: Vec<Output> = Vec::new();
        let placeholder_string_no_url = String::from("");
        for document in documents.documents {
            let url = match &document.url {
                Some(value) => &value,
                None => &placeholder_string_no_url, // potentially undefined behavior; TODO: document
            };
            if !(&self.scope.pattern.quick_check(url)) {
                continue; // scope doesn't match
            }
            let input = document.content(&self.scope);
            let mut matches: HashMap<&String, bool> = HashMap::new();
            let mut match_results: Vec<PatternMatch> = Vec::new();
            for trigger in &self.triggers {
                let does_match = trigger.quick_check(&input);
                if does_match {
                    match_results.push(match trigger.full_check(&input) {
                        Some(value) => value,
                        None => continue, // no match on this trigger...but there was earlier?
                    });
                }
                matches.insert(&trigger.id, does_match);
            }
            if match self.threshold.evaluate(&matches) {
                Ok(evaluation) => evaluation,
                Err(_) => continue, // TODO: make this not fail silently
            } {
                outputs.push(assemble(&document, &self, match_results, None));
            }
        }
        OutputBatch::from(outputs)
    }
}