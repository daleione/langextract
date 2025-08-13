/*!
Provides functionality for annotating medical text using a language model.

The annotation process involves tokenizing the input text, generating prompts
for the language model, and resolving the language model's output into
structured annotations.

Usage example:
    let annotator = Annotator::new(language_model, prompt_template);
    let annotated_documents = annotator.annotate_documents(documents, resolver);
*/

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::chunking::{ChunkIterator, TextChunk, make_batches_of_textchunk};
use crate::data::{AnnotatedDocument, Document, Extraction, FormatType};
use crate::inference::{BaseLanguageModel, InferenceOutputError};
use crate::progress;
use crate::prompting::{PromptTemplateStructured, QAPromptGenerator};
use crate::resolver::AbstractResolver;

const ATTRIBUTE_SUFFIX: &str = "_attributes";

/// Exception raised when identical document ids are present.
#[derive(Debug, Clone)]
pub struct DocumentRepeatError(pub String);

impl std::fmt::Display for DocumentRepeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DocumentRepeatError: {}", self.0)
    }
}
impl std::error::Error for DocumentRepeatError {}

/// Merges extractions from multiple extraction passes.
/// When extractions from different passes overlap in their character positions,
/// the extraction from the earlier pass is kept (first-pass wins strategy).
/// Only non-overlapping extractions from later passes are added to the result.
pub fn merge_non_overlapping_extractions(all_extractions: &[Vec<Extraction>]) -> Vec<Extraction> {
    if all_extractions.is_empty() {
        return vec![];
    }
    if all_extractions.len() == 1 {
        return all_extractions[0].clone();
    }
    let mut merged_extractions = all_extractions[0].clone();
    for pass_extractions in &all_extractions[1..] {
        for extraction in pass_extractions {
            let mut overlaps = false;
            if let Some(ref _interval) = extraction.char_interval {
                for existing_extraction in &merged_extractions {
                    if let Some(ref _existing_interval) = existing_extraction.char_interval {
                        if extractions_overlap(extraction, existing_extraction) {
                            overlaps = true;
                            break;
                        }
                    }
                }
            }
            if !overlaps {
                merged_extractions.push(extraction.clone());
            }
        }
    }
    merged_extractions
}

/// Checks if two extractions overlap based on their character intervals.
pub fn extractions_overlap(extraction1: &Extraction, extraction2: &Extraction) -> bool {
    let (start1, end1) = match &extraction1.char_interval {
        Some(interval) => (interval.start_pos, interval.end_pos),
        None => return false,
    };
    let (start2, end2) = match &extraction2.char_interval {
        Some(interval) => (interval.start_pos, interval.end_pos),
        None => return false,
    };
    // Two intervals overlap if one starts before the other ends
    start1 < end2 && start2 < end1
}

/// Iterates over documents to yield text chunks along with the document ID.
/// Restricts repeats if specified.
pub fn document_chunk_iterator(
    documents: Vec<Document>,
    max_char_buffer: usize,
    restrict_repeats: bool,
) -> Result<Vec<TextChunk>, DocumentRepeatError> {
    let mut visited_ids = HashSet::new();
    let mut chunks = Vec::new();
    for mut document in documents {
        let document_id = document.document_id();
        if restrict_repeats && visited_ids.contains(&document_id) {
            return Err(DocumentRepeatError(format!(
                "Document id {} is already visited.",
                document_id
            )));
        }
        let tokenized_text = document.tokenized_text().clone();
        let chunk_iter = ChunkIterator::new(&tokenized_text, max_char_buffer, Some(document.clone()));
        visited_ids.insert(document_id);
        for chunk in chunk_iter {
            chunks.push(chunk);
        }
    }
    Ok(chunks)
}

/// Annotates documents with extractions using a language model.
pub struct Annotator<L: BaseLanguageModel> {
    language_model: L,
    prompt_generator: QAPromptGenerator,
}

impl<L: BaseLanguageModel> Annotator<L> {
    /// Initializes Annotator.
    pub fn new(
        language_model: L,
        prompt_template: PromptTemplateStructured,
        format_type: FormatType,
        attribute_suffix: Option<&str>,
        fence_output: bool,
    ) -> Self {
        let mut prompt_generator = QAPromptGenerator::new(prompt_template);
        prompt_generator.format_type = crate::prompting::FormatType::try_from(match format_type {
            crate::data::FormatType::Yaml => "yaml",
            crate::data::FormatType::Json => "json",
        })
        .unwrap_or(crate::prompting::FormatType::YAML);
        prompt_generator.attribute_suffix = attribute_suffix.unwrap_or(ATTRIBUTE_SUFFIX).to_string();
        prompt_generator.fence_output = fence_output;
        println!("Initialized Annotator with prompt:\n{:?}", prompt_generator);
        Self {
            language_model,
            prompt_generator,
        }
    }

    /// Annotates a sequence of documents with NLP extractions.
    /// Breaks documents into chunks, processes them into prompts and performs
    /// batched inference, mapping annotated extractions back to the original document.
    pub fn annotate_documents(
        &self,
        documents: Vec<Document>,
        resolver: &dyn AbstractResolver,
        max_char_buffer: usize,
        batch_length: usize,
        debug: bool,
        extraction_passes: usize,
        extra_args: Option<HashMap<String, String>>,
    ) -> Result<Vec<AnnotatedDocument>, InferenceOutputError> {
        if extraction_passes == 1 {
            self.annotate_documents_single_pass(documents, resolver, max_char_buffer, batch_length, debug, extra_args)
        } else {
            self.annotate_documents_sequential_passes(
                documents,
                resolver,
                max_char_buffer,
                batch_length,
                debug,
                extraction_passes,
                extra_args,
            )
        }
    }

    /// Single-pass annotation logic (original implementation).
    fn annotate_documents_single_pass(
        &self,
        documents: Vec<Document>,
        resolver: &dyn AbstractResolver,
        max_char_buffer: usize,
        batch_length: usize,
        debug: bool,
        _extra_args: Option<HashMap<String, String>>,
    ) -> Result<Vec<AnnotatedDocument>, InferenceOutputError> {
        println!("Starting document annotation.");
        let mut docs: Vec<Document> = documents;
        let chunk_iter = document_chunk_iterator(docs.clone(), max_char_buffer, true)
            .map_err(|e| InferenceOutputError::new(e.to_string()))?;
        let mut doc_iter = docs.iter_mut();
        let mut curr_document = doc_iter.next();
        if curr_document.is_none() {
            println!("No documents to process.");
            return Ok(vec![]);
        }
        let mut annotated_extractions: Vec<Extraction> = Vec::new();
        let batches = make_batches_of_textchunk(chunk_iter.into_iter(), batch_length);
        let model_info = None; // progress::get_model_info(&self.language_model);
        let mut chars_processed = 0;
        let mut annotated_documents = Vec::new();

        for (index, mut batch) in batches.into_iter().enumerate() {
            println!("Processing batch {} with length {}", index, batch.len());
            let batch_prompts: Vec<String> = batch
                .iter_mut()
                .map(|text_chunk| {
                    self.prompt_generator
                        .render(&text_chunk.chunk_text().unwrap_or_default())
                })
                .collect();

            // Show what we're currently processing
            if debug {
                let batch_size: usize = batch
                    .iter_mut()
                    .map(|chunk| chunk.chunk_text().unwrap_or_default().len())
                    .sum();
                let _desc = progress::format_extraction_progress(model_info, Some(batch_size), Some(chars_processed));
                // progress bar description update not implemented
            }

            // infer is async, so we need to block here for demonstration (in real code, use async/await)
            let batch_scored_outputs = futures::executor::block_on(self.language_model.infer(&batch_prompts, None))?;

            // Update total processed
            if debug {
                for mut chunk in batch.clone() {
                    if let Some(_document_text) = chunk.document_text() {
                        if let Ok(char_interval) = chunk.char_interval() {
                            let start = char_interval.start_pos.unwrap_or(0);
                            let end = char_interval.end_pos.unwrap_or(0);
                            chars_processed += end - start;
                        }
                    }
                }
                let batch_size: usize = batch
                    .iter_mut()
                    .map(|chunk| chunk.chunk_text().unwrap_or_default().len())
                    .sum();
                let _desc = progress::format_extraction_progress(model_info, Some(batch_size), Some(chars_processed));
                // progress bar description update not implemented
            }

            for (text_chunk, scored_outputs) in batch.into_iter().zip(batch_scored_outputs.iter()) {
                println!("Processing chunk: {:?}", text_chunk);
                if scored_outputs.is_empty() {
                    println!("No scored outputs for chunk with ID {:?}.", text_chunk.document_id());
                    return Err(InferenceOutputError::new("No scored outputs from language model."));
                }
                let current_doc_id = curr_document.as_mut().map(|d| d.document_id());
                while current_doc_id != text_chunk.document_id() {
                    let doc_id = curr_document.as_mut().map(|d| d.document_id());
                    println!("Completing annotation for document ID {:?}.", doc_id);
                    let annotated_doc = AnnotatedDocument::new(
                        curr_document.as_mut().map(|d| Some(d.document_id())).unwrap_or(None),
                        Some(annotated_extractions.clone()),
                        Some(curr_document.as_mut().map(|d| d.text.clone()).unwrap_or_default()),
                    );
                    annotated_documents.push(annotated_doc);
                    annotated_extractions.clear();
                    curr_document = doc_iter.next();
                    assert!(
                        curr_document.is_some(),
                        "Document should be defined for chunk per document_chunk_iterator specifications."
                    );
                }
                let top_inference_result = scored_outputs[0].output.clone().unwrap_or_default();
                println!("Top inference result: {}", top_inference_result);

                let annotated_chunk_extractions = resolver.resolve(&top_inference_result, debug);

                // Get all values that need mutable access first
                let mut text_chunk_for_text = text_chunk.clone();
                let chunk_text = text_chunk_for_text.chunk_text().unwrap_or_default();

                let mut text_chunk_for_char = text_chunk.clone();
                let char_offset = match text_chunk_for_char.char_interval() {
                    Ok(ci) => ci.start_pos.unwrap_or(0),
                    Err(_) => 0,
                };

                // Get immutable values
                let token_offset = text_chunk.token_interval.start_index;

                // For demonstration, use default values for fuzzy alignment
                let enable_fuzzy_alignment = false;
                let fuzzy_alignment_threshold = 0.75;
                let accept_match_lesser = false;

                let aligned_extractions = match &annotated_chunk_extractions {
                    Ok(extractions) => resolver.align(
                        extractions,
                        &chunk_text,
                        token_offset,
                        Some(char_offset),
                        enable_fuzzy_alignment,
                        fuzzy_alignment_threshold,
                        accept_match_lesser,
                    ),
                    Err(_) => Vec::new(),
                };
                annotated_extractions.extend(aligned_extractions.into_iter().map(|e| {
                    let token_interval = e.token_interval.map(|ti| crate::tokenizer::TokenInterval {
                        start_index: ti.start_index,
                        end_index: ti.end_index,
                    });
                    let char_interval = e.char_interval.map(|ci| crate::data::CharInterval {
                        start_pos: Some(ci.start_pos),
                        end_pos: Some(ci.end_pos),
                    });
                    let alignment_status = e.alignment_status.map(|status| match status {
                        crate::resolver::data::AlignmentStatus::MatchExact => crate::data::AlignmentStatus::MatchExact,
                        crate::resolver::data::AlignmentStatus::MatchLesser => {
                            crate::data::AlignmentStatus::MatchLesser
                        }
                        crate::resolver::data::AlignmentStatus::MatchFuzzy => crate::data::AlignmentStatus::MatchFuzzy,
                    });
                    crate::data::Extraction::new(
                        e.extraction_class.clone(),
                        e.extraction_text.clone(),
                        token_interval,
                        char_interval,
                        alignment_status,
                        Some(e.extraction_index),
                        Some(e.group_index),
                        None,
                        None,
                    )
                }));
            }
        }
        progress::print_extraction_complete();
        if let Some(curr_document) = curr_document {
            println!("Finalizing annotation for document ID {}.", curr_document.document_id());
            let annotated_doc = AnnotatedDocument::new(
                Some(curr_document.document_id()),
                Some(annotated_extractions.clone()),
                Some(curr_document.text.clone()),
            );
            annotated_documents.push(annotated_doc);
        }
        println!("Document annotation completed.");
        Ok(annotated_documents)
    }

    /// Sequential extraction passes logic for improved recall.
    fn annotate_documents_sequential_passes(
        &self,
        documents: Vec<Document>,
        resolver: &dyn AbstractResolver,
        max_char_buffer: usize,
        batch_length: usize,
        debug: bool,
        extraction_passes: usize,
        extra_args: Option<HashMap<String, String>>,
    ) -> Result<Vec<AnnotatedDocument>, InferenceOutputError> {
        println!(
            "Starting sequential extraction passes for improved recall with {} passes.",
            extraction_passes
        );
        let document_list: Vec<Document> = documents;
        let mut document_extractions_by_pass: HashMap<String, Vec<Vec<Extraction>>> = HashMap::new();
        let mut document_texts: HashMap<String, String> = HashMap::new();

        for pass_num in 0..extraction_passes {
            println!("Starting extraction pass {} of {}", pass_num + 1, extraction_passes);
            let annotated_docs = self.annotate_documents_single_pass(
                document_list.clone(),
                resolver,
                max_char_buffer,
                batch_length,
                debug && pass_num == 0,
                extra_args.clone(),
            )?;
            for mut annotated_doc in annotated_docs {
                let doc_id = annotated_doc.document_id().clone();
                document_extractions_by_pass
                    .entry(doc_id.clone())
                    .or_default()
                    .push(annotated_doc.extractions.clone().unwrap_or_default());
                document_texts
                    .entry(doc_id.clone())
                    .or_insert(annotated_doc.text.clone().unwrap_or_default());
            }
        }

        let mut results = Vec::new();
        for (doc_id, all_pass_extractions) in document_extractions_by_pass.iter() {
            let merged_extractions = merge_non_overlapping_extractions(all_pass_extractions);
            if debug {
                let total_extractions: usize = all_pass_extractions.iter().map(|extractions| extractions.len()).sum();
                println!(
                    "Document {}: Merged {} extractions from {} passes into {} non-overlapping extractions.",
                    doc_id,
                    total_extractions,
                    extraction_passes,
                    merged_extractions.len(),
                );
            }
            results.push(AnnotatedDocument::new(
                Some(doc_id.clone()),
                Some(merged_extractions),
                Some(document_texts.get(doc_id).cloned().unwrap_or_default()),
            ));
        }
        println!("Sequential extraction passes completed.");
        Ok(results)
    }

    /// Annotates text with NLP extractions for text input.
    pub fn annotate_text(
        &self,
        text: &str,
        resolver: &dyn AbstractResolver,
        max_char_buffer: usize,
        batch_length: usize,
        additional_context: Option<&str>,
        debug: bool,
        extraction_passes: usize,
        extra_args: Option<HashMap<String, String>>,
    ) -> Result<AnnotatedDocument, InferenceOutputError> {
        let start_time = if debug { Some(Instant::now()) } else { None };
        let document = Document::new(text.to_string(), None, additional_context.map(|s| s.to_string()));
        let mut annotations = self.annotate_documents(
            vec![document],
            resolver,
            max_char_buffer,
            batch_length,
            debug,
            extraction_passes,
            extra_args,
        )?;
        assert_eq!(
            annotations.len(),
            1,
            "Expected 1 annotation but got {} annotations.",
            annotations.len()
        );
        if debug && annotations[0].extractions.as_ref().map_or(false, |v| !v.is_empty()) {
            let elapsed_time = start_time.map(|t| t.elapsed().as_secs_f64());
            let num_extractions = annotations[0].extractions.as_ref().map_or(0, |v| v.len());
            let unique_classes = annotations[0].extractions.as_ref().map_or(0, |v| {
                v.iter().map(|e| &e.extraction_class).collect::<HashSet<_>>().len()
            });
            let num_chunks = text.len() / max_char_buffer + if text.len() % max_char_buffer > 0 { 1 } else { 0 };
            progress::print_extraction_summary(
                num_extractions,
                unique_classes,
                elapsed_time,
                Some(text.len()),
                Some(num_chunks),
            );
        }
        Ok(AnnotatedDocument::new(
            Some(annotations[0].document_id()),
            annotations[0].extractions.clone(),
            annotations[0].text.clone(),
        ))
    }
}
