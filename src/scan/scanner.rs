//! This file provides functionality related to scanning.

use common::compilation::CompilableTo;
use common::pattern::PatternMatch;
use common::retrieve::load_document;
use input::document::{
    CompiledDocument, CompiledDocumentBatch, Document, DocumentBatch, DocumentReference,
    DocumentReferenceBatch,
};
use output::output::{Output, OutputBatch};
use query::query::{CompiledQuery, CompiledQueryGroup};
use std::collections::{HashMap, HashSet};

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This trait specifies basic scanning functionality.
pub trait Scanner: Clone + Send {
    /// Scan a batch of documents and return the output. This function
    /// is **singlethreaded** and often not very performant.
    fn scan_batch(&self, documents: &CompiledDocumentBatch) -> OutputBatch;
    /// Scan a single document and return the output.
    fn scan_single(&self, document: &CompiledDocument) -> OutputBatch;
    /// Launch a 'scan engine' and create an asynchronous and concurrent
    /// scanning system. In most cases, this is what you'll want to use.
    ///
    /// For more information about how to interact with the scanning system
    /// (sometimes referred to as the _scan engine_), please see the documentation
    /// pertaining to `AsyncScanInterface`.
    fn scan_concurrently(&self, threads: u8) -> AsyncScanInterface;
}

/// `AsyncScanInterface` provides a simple interface, free of channels
/// and other complicated components, to communicate with the scan engine.
pub struct AsyncScanInterface {
    outgoing_batches: Option<mpsc::Sender<DocumentReferenceBatch>>,
    incoming_outputs: mpsc::Receiver<OutputBatch>,
    pending_processing: Arc<Mutex<usize>>,
}

impl AsyncScanInterface {
    /// Process the given documents. Note that this will temporarily lock
    /// the thread in order to increment the number of items processing.
    pub fn process(&self, batch: DocumentReferenceBatch) -> Result<(), ()> {
        match &self.outgoing_batches {
            Some(value) => match value.send(batch) {
                Ok(_) => {
                    *self.pending_processing.lock().unwrap() += 1;
                    Ok(())
                }
                Err(error) => Err(()),
            },
            None => Err(()),
        }
    }

    /// Lock the current thread and wait for outputs.
    pub fn lock_for_outputs(&self) -> Result<OutputBatch, mpsc::RecvError> {
        self.incoming_outputs.recv()
    }

    /// Lock the current thread and determine the total number of batches
    /// that are currently processing (i.e. the total size of the current
    /// inter-thread queue).
    pub fn batches_pending_processing(&self) -> usize {
        self.pending_processing.lock().unwrap().clone() // unsafe?
    }

    /// Signal to the scan engine to shut down. Sending documents
    /// will no longer be possible.
    pub fn shutdown(&mut self) {
        self.outgoing_batches = None;
    }
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

    fn scan_concurrently(&self, threads: u8) -> AsyncScanInterface {
        let query_group = CompiledQueryGroup::from(self.clone());
        query_group.scan_concurrently(threads)
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

    fn scan_concurrently(&self, threads: u8) -> AsyncScanInterface {
        let (incoming_transmitter, incoming_receiver) = mpsc::channel::<DocumentReferenceBatch>();
        let pending_processing = Arc::new(Mutex::new(0));

        // println!("scanning concurrently");
        let (ultimate_transmitter, ultimate_receiver) = mpsc::channel::<OutputBatch>();
        let cloned_self = self.clone();
        let pending_processing_cloned = pending_processing.clone();

        thread::spawn(move || {
            let (tx_requests, rx_requests) = mpsc::channel::<thread::ThreadId>();
            let mut handles: Vec<thread::JoinHandle<_>> = Vec::new();
            let mut outgoing: HashMap<thread::ThreadId, mpsc::Sender<DocumentReferenceBatch>> =
                HashMap::new();

            // create threads
            for _ in 0..threads {
                let (tx_inputs, rx_inputs) = mpsc::channel::<DocumentReferenceBatch>();
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
                        // println!("found batch of {} document on {:?}", batch.documents.len(), id);
                        let mut documents: Vec<Document> = Vec::new();
                        for document_reference in batch.documents {
                            documents.push(match document_reference {
                                DocumentReference::Populated(document) => document,
                                DocumentReference::Unpopulated(path) => {
                                    match load_document(&path) {
                                        Ok(document) => document,
                                        Err(_issue) => {
                                            // println!("{}", issue);
                                            continue;
                                        } // silent failure
                                    }
                                }
                            });
                        }
                        let populated_batch = DocumentBatch::from(documents);
                        let compiled_batch = match populated_batch.compile() {
                            Ok(value) => value,
                            Err(_) => continue, // silent failure; TODO: fix
                        };
                        let outputs = supercloned_self.scan_batch(&compiled_batch);
                        // println!("sending {} outputs...", outputs.outputs.len());
                        match tx_send_output.send(outputs) {
                            Ok(_) => (),
                            Err(_) => break, // receiver has been killed; thread is done
                        };
                    }
                    drop(tx_send_output);
                });
                outgoing.insert(handle.thread().id(), tx_inputs);
                handles.push(handle);
            }

            // listen and coordinate threads
            // TODO: figure out how to deal with these silent failures
            loop {
                let request = match rx_requests.recv() {
                    Ok(request) => request,
                    Err(_error) => break,
                };
                let batch_to_send = match incoming_receiver.recv() {
                    Ok(batch) => {
                        *pending_processing_cloned.lock().unwrap() -= 1;
                        batch
                    }
                    Err(error) => {
                        drop(rx_requests);
                        break;
                    } // we're done; transmitter dropped
                };
                match outgoing.get(&request) {
                    Some(channel) => {
                        match channel.send(batch_to_send) {
                            Ok(_) => (),
                            Err(_) => continue, // silent failure
                        };
                    }
                    None => break, // silent failure
                }
                // decrement pending processing
            }

            // Thread clean-up
            for outgoing_sender in outgoing.values() {
                drop(outgoing_sender);
            }
        });
        AsyncScanInterface {
            incoming_outputs: ultimate_receiver,
            outgoing_batches: Some(incoming_transmitter),
            pending_processing: pending_processing,
        }
    }
}
