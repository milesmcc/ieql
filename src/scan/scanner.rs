use common::pattern::PatternMatch;
use input::document::{CompiledDocument, CompiledDocumentBatch, DocumentBatch};
use output::output::{Output, OutputBatch};
use query::query::{CompiledQuery, CompiledQueryGroup};
use std::collections::{HashMap, HashSet};
use common::compilation::CompilableTo;

use std::sync::mpsc;
use std::thread;

pub trait Scanner: Clone + Send {
    fn scan_batch(&self, documents: &CompiledDocumentBatch) -> OutputBatch;
    fn scan_single(&self, document: &CompiledDocument) -> OutputBatch;
    fn scan_concurrently(
        &self,
        batches: mpsc::Receiver<DocumentBatch>,
        threads: u8,
    ) -> mpsc::Receiver<OutputBatch>;
}

impl Scanner for CompiledQuery {
    fn scan_single(&self, document: &CompiledDocument) -> OutputBatch {
        let placeholder_string_no_url = String::from("");
        let url = match &document.url {
            Some(value) => &value,
            None => &placeholder_string_no_url, // potentially undefined behavior; TODO: document
        };
        if !(&self.scope.pattern.quick_check(url)) {
            return OutputBatch::from(vec![]); // scope doesn't match; TODO: optimize this so that this function is only called in the first place on things that match
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

    fn scan_batch(&self, documents: &CompiledDocumentBatch) -> OutputBatch {
        let mut outputs: Vec<Output> = Vec::new();
        for document in &documents.documents {
            let output_batch = self.scan_single(document);
            outputs.extend(output_batch.outputs);
        }
        OutputBatch::from(outputs)
    }

    fn scan_concurrently(
        &self,
        batches: mpsc::Receiver<DocumentBatch>,
        threads: u8,
    ) -> mpsc::Receiver<OutputBatch> {
        let query_group = CompiledQueryGroup::from(self.clone());
        query_group.scan_concurrently(batches, threads)
    }
}

impl Scanner for CompiledQueryGroup {
    fn scan_single(&self, document: &CompiledDocument) -> OutputBatch {
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

    fn scan_batch(&self, documents: &CompiledDocumentBatch) -> OutputBatch {
        let mut output_batch = OutputBatch::from(vec![]);
        for document in &documents.documents {
            output_batch.merge_with(self.scan_single(document));
        }
        output_batch
    }

    fn scan_concurrently(
        &self,
        batches: mpsc::Receiver<DocumentBatch>,
        threads: u8,
    ) -> mpsc::Receiver<OutputBatch> {
        let (ultimate_transmitter, ultimate_receiver) = mpsc::channel::<OutputBatch>();
        let cloned_self = self.clone();
        thread::spawn(move || {
            let (tx_requests, rx_requests) = mpsc::channel::<thread::ThreadId>();
            let mut handles: Vec<thread::JoinHandle<_>> = Vec::new();
            let mut outgoing: HashMap<thread::ThreadId, mpsc::Sender<DocumentBatch>> =
                HashMap::new();

            // create threads
            for _ in 0..threads {
                let (tx_inputs, rx_inputs) = mpsc::channel::<DocumentBatch>();
                let tx_request_documents = tx_requests.clone();
                let tx_send_output = ultimate_transmitter.clone();
                let supercloned_self = cloned_self.clone(); // TODO: optimize
                let handle = thread::spawn(move || {
                    let id = thread::current().id();
                    loop {
                        match tx_request_documents.send(id) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                        let batch = match rx_inputs.recv() {
                            Ok(values) => values,
                            Err(_) => break, // no more values; end the thread
                        };
                        let compiled_batch = match batch.compile() {
                            Ok(value) => value,
                            Err(_) => break, // silent failure; TODO: fix
                        };
                        let outputs = supercloned_self.scan_batch(&compiled_batch);
                        match tx_send_output.send(outputs) {
                            Ok(_) => (),
                            Err(_) => break, // receiver has been killed; thread is done
                        };
                    }
                });
                outgoing.insert(handle.thread().id(), tx_inputs);
                handles.push(handle);
            }

            // listen and coordinate threads
            // TODO: figure out how to deal with these silent failures
            loop {
                let request = match rx_requests.recv() {
                    Ok(request) => request,
                    Err(error) => {
                        break; // silent failure
                    }
                };
                let batch_to_send = match batches.recv() {
                    Ok(batch) => batch,
                    Err(_) => break, // we're done; transmitter dropped
                };
                match outgoing.get(&request) {
                    Some(channel) => {
                        channel.send(batch_to_send);
                    }
                    None => break, // silent failure
                }
            }
        });
        ultimate_receiver
    }
}
