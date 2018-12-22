use common::pattern::PatternMatch;
use input::document::{Document, DocumentBatch};
use output::output::{Output, OutputBatch};
use query::query::{CompiledQuery, CompiledQueryGroup};
use std::collections::{HashMap, HashSet};

pub trait Scanner {
    fn scan_batch(&self, documents: &DocumentBatch) -> OutputBatch;
    fn scan_single(&self, document: &Document) -> OutputBatch;
}

impl Scanner for CompiledQuery {
    fn scan_single(&self, document: &Document) -> OutputBatch {
        let placeholder_string_no_url = String::from("");
        let url = match &document.url {
            Some(value) => &value,
            None => &placeholder_string_no_url, // potentially undefined behavior; TODO: document
        };
        if !(&self.scope.pattern.quick_check(url)) {
            return OutputBatch::from(vec![]); // scope doesn't match
        }
        let input = document.content(self.scope.content);
        let mut matches: HashMap<&String, bool> = HashMap::new();
        let mut match_results: Vec<PatternMatch> = Vec::new();
        for trigger in &self.triggers {
            let does_match = trigger.quick_check(&input);
            if does_match {
                match_results.push(match trigger.full_check(&input) {
                    Some(value) => value,
                    None => return OutputBatch::from(vec![]), // no match on this trigger...but there was earlier?
                });
            }
            matches.insert(&trigger.id, does_match);
        }
        if match self.threshold.evaluate(&matches) {
            Ok(evaluation) => evaluation,
            Err(_) => return OutputBatch::from(vec![]), // TODO: make this not fail silently
        } {
            return OutputBatch::from(vec![Output::new(&document, &self, match_results, None)]);
        } else {
            return OutputBatch::from(vec![]);
        }
    }

    fn scan_batch(&self, documents: &DocumentBatch) -> OutputBatch {
        let mut outputs: Vec<Output> = Vec::new();
        for document in &documents.documents {
            let output_batch = self.scan_single(document);
            outputs.extend(output_batch.outputs);
        }
        OutputBatch::from(outputs)
    }
}

impl Scanner for CompiledQueryGroup {
    fn scan_single(&self, document: &Document) -> OutputBatch {
        let mut output_batch = OutputBatch::new();

        // Regex Set evaluation
        let to_feed = document.content(self.regex_feed);
        let matches: Vec<_> = self.regex_collected.matches(to_feed).into_iter().collect();
        let mut queries_to_run: HashSet<usize> = HashSet::new();
        for match_item in matches {
            queries_to_run.insert(match self.regex_collected_query_index.get(match_item) {
                Some(index) => *index,
                None => return OutputBatch::from(vec![]), // this should never happen; should we panic? TODO
            });
        }
        for query_index in queries_to_run {
            let query = match self.queries.get(query_index) {
                Some(value) => value,
                None => return OutputBatch::from(vec![]), // this should also never happen; should we panic? TODO
            };
            output_batch.merge_with(query.scan_single(document));
        }

        // Always runs
        for query in &self.always_run_queries {
            output_batch.merge_with(query.scan_single(document));
        }

        output_batch
    }

    fn scan_batch(&self, documents: &DocumentBatch) -> OutputBatch {
        let mut output_batch = OutputBatch::from(vec![]);
        for document in &documents.documents {
            output_batch.merge_with(self.scan_single(document));
        }
        output_batch
    }
}
