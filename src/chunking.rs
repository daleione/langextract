//! Library for breaking documents into chunks of sentences.
//!
//! When a text-to-text model (e.g. a large language model with a fixed context
//! size) cannot accommodate a large document, this library can help us break the
//! document into chunks of a required maximum length that we can perform
//! inference on.

use std::fmt;
use std::iter::Peekable;

use crate::data::{CharInterval, Document};
use crate::tokenizer::{self, TokenInterval, TokenizedText};

/// Error raised when token_util returns unexpected values.
#[derive(Debug, Clone)]
pub struct TokenUtilError(pub String);

impl fmt::Display for TokenUtilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TokenUtilError: {}", self.0)
    }
}

impl std::error::Error for TokenUtilError {}

/// Stores a text chunk with attributes to the source document.
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub token_interval: TokenInterval,
    pub document: Option<Document>,
    chunk_text: Option<String>,
    sanitized_chunk_text: Option<String>,
    char_interval: Option<CharInterval>,
}

impl TextChunk {
    pub fn new(token_interval: TokenInterval, document: Option<Document>) -> Self {
        Self {
            token_interval,
            document,
            chunk_text: None,
            sanitized_chunk_text: None,
            char_interval: None,
        }
    }

    /// Gets the document ID from the source document.
    pub fn document_id(&self) -> Option<String> {
        self.document.as_ref().map(|doc| {
            let mut_doc = &mut doc.clone();
            mut_doc.document_id()
        })
    }

    /// Gets the tokenized text from the source document.
    pub fn document_text(&self) -> Option<TokenizedText> {
        self.document.as_ref().map(|doc| {
            let mut_doc = &mut doc.clone();
            mut_doc.tokenized_text().clone()
        })
    }

    /// Gets the chunk text. Raises an error if `document_text` is not set.
    pub fn chunk_text(&mut self) -> Result<&str, TokenUtilError> {
        if self.document_text().is_none() {
            return Err(TokenUtilError(
                "document_text must be set to access chunk_text.".to_string(),
            ));
        }
        if self.chunk_text.is_none() {
            let txt = get_token_interval_text(&self.document_text().unwrap(), &self.token_interval)?;
            self.chunk_text = Some(txt);
        }
        Ok(self.chunk_text.as_ref().unwrap())
    }

    /// Gets the sanitized chunk text.
    pub fn sanitized_chunk_text(&mut self) -> Result<&str, TokenUtilError> {
        if self.sanitized_chunk_text.is_none() {
            let txt = self.chunk_text()?;
            let sanitized = sanitize(txt)?;
            self.sanitized_chunk_text = Some(sanitized);
        }
        Ok(self.sanitized_chunk_text.as_ref().unwrap())
    }

    /// Gets the additional context for prompting from the source document.
    pub fn additional_context(&self) -> Option<&str> {
        self.document.as_ref().and_then(|doc| doc.additional_context.as_deref())
    }

    /// Gets the character interval corresponding to the token interval.
    pub fn char_interval(&mut self) -> Result<&CharInterval, TokenUtilError> {
        if self.char_interval.is_none() {
            if self.document_text().is_none() {
                return Err(TokenUtilError(
                    "document_text must be set to compute char_interval.".to_string(),
                ));
            }
            let interval = get_char_interval(&self.document_text().unwrap(), &self.token_interval)?;
            self.char_interval = Some(interval);
        }
        Ok(self.char_interval.as_ref().unwrap())
    }
}

impl fmt::Display for TextChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let interval_repr = format!(
            "start_index: {}, end_index: {}",
            self.token_interval.start_index, self.token_interval.end_index
        );
        let doc_id_repr = match self.document_id() {
            Some(id) => format!("Document ID: {}", id),
            None => "Document ID: None".to_string(),
        };
        let chunk_text_repr = match &self.chunk_text {
            Some(txt) => format!("'{}'", txt),
            None => "<unavailable: document_text not set>".to_string(),
        };
        write!(
            f,
            "TextChunk(\n  interval=[{}],\n  {},\n  Chunk Text: {}\n)",
            interval_repr, doc_id_repr, chunk_text_repr
        )
    }
}

/// Creates a token interval.
pub fn create_token_interval(start_index: usize, end_index: usize) -> Result<TokenInterval, TokenUtilError> {
    if start_index >= end_index {
        return Err(TokenUtilError(format!(
            "Start index {} must be < end index {}.",
            start_index, end_index
        )));
    }
    Ok(TokenInterval { start_index, end_index })
}

/// Get the text within an interval of tokens.
pub fn get_token_interval_text(
    tokenized_text: &TokenizedText,
    token_interval: &TokenInterval,
) -> Result<String, TokenUtilError> {
    if token_interval.start_index >= token_interval.end_index {
        return Err(TokenUtilError(format!(
            "Start index {} must be < end index {}.",
            token_interval.start_index, token_interval.end_index
        )));
    }
    let return_string =
        tokenizer::tokens_text(tokenized_text, token_interval).map_err(|e| TokenUtilError(e.to_string()))?;
    println!(
        "Token util returns string: {} for tokenized_text: {:?}, token_interval: {:?}",
        return_string, tokenized_text, token_interval
    );
    if !tokenized_text.text.is_empty() && return_string.is_empty() {
        return Err(TokenUtilError(format!(
            "Token util returns an empty string unexpectedly. Number of tokens is tokenized_text: {}, token_interval is {} to {}, which should not lead to empty string.",
            tokenized_text.tokens.len(),
            token_interval.start_index,
            token_interval.end_index
        )));
    }
    Ok(return_string)
}

// Use tokenizer module's implementation
pub fn tokens_text(tokenized_text: &TokenizedText, token_interval: &TokenInterval) -> Result<String, TokenUtilError> {
    tokenizer::tokens_text(tokenized_text, token_interval).map_err(|e| TokenUtilError(e.to_string()))
}

/// Returns the char interval corresponding to the token interval.
pub fn get_char_interval(
    tokenized_text: &TokenizedText,
    token_interval: &TokenInterval,
) -> Result<CharInterval, TokenUtilError> {
    if token_interval.start_index >= token_interval.end_index {
        return Err(TokenUtilError(format!(
            "Start index {} must be < end index {}.",
            token_interval.start_index, token_interval.end_index
        )));
    }
    let start_token = &tokenized_text.tokens[token_interval.start_index];
    let final_token = &tokenized_text.tokens[token_interval.end_index - 1];
    Ok(CharInterval {
        start_pos: Some(start_token.char_interval.start_pos),
        end_pos: Some(final_token.char_interval.end_pos),
    })
}

/// Converts all whitespace characters in input text to a single space.
pub fn sanitize(text: &str) -> Result<String, TokenUtilError> {
    let sanitized_text = text.trim().split_whitespace().collect::<Vec<_>>().join(" ");
    if sanitized_text.is_empty() {
        return Err(TokenUtilError("Sanitized text is empty.".to_string()));
    }
    Ok(sanitized_text)
}

/// Processes chunks into batches of TextChunk for inference.
pub fn make_batches_of_textchunk<I>(chunk_iter: I, batch_length: usize) -> Vec<Vec<TextChunk>>
where
    I: Iterator<Item = TextChunk>,
{
    let vec: Vec<_> = chunk_iter.collect();
    vec.chunks(batch_length).map(|chunk| chunk.to_vec()).collect()
}

/// Iterate through sentences of a tokenized text.
pub struct SentenceIterator<'a> {
    tokenized_text: &'a TokenizedText,
    token_len: usize,
    curr_token_pos: usize,
}

impl<'a> SentenceIterator<'a> {
    pub fn new(tokenized_text: &'a TokenizedText, curr_token_pos: usize) -> Result<Self, TokenUtilError> {
        let token_len = tokenized_text.tokens.len();
        if curr_token_pos > token_len {
            return Err(TokenUtilError(format!(
                "Current token position {} is past the length of the document {}.",
                curr_token_pos, token_len
            )));
        }
        Ok(Self {
            tokenized_text,
            token_len,
            curr_token_pos,
        })
    }
}

impl<'a> Iterator for SentenceIterator<'a> {
    type Item = TokenInterval;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_token_pos == self.token_len {
            return None;
        }
        let sentence_range = tokenizer::find_sentence_range(
            &self.tokenized_text.text,
            &self.tokenized_text.tokens,
            self.curr_token_pos,
        )
        .ok()?;
        let sentence_range = TokenInterval {
            start_index: self.curr_token_pos,
            end_index: sentence_range.end_index,
        };
        self.curr_token_pos = sentence_range.end_index;
        Some(sentence_range)
    }
}

/// Iterate through chunks of a tokenized text.
pub struct ChunkIterator<'a> {
    tokenized_text: &'a TokenizedText,
    max_char_buffer: usize,
    sentence_iter: Peekable<SentenceIterator<'a>>,
    document: Document,
    broken_sentence: bool,
}

impl<'a> ChunkIterator<'a> {
    pub fn new(text: &'a TokenizedText, max_char_buffer: usize, document: Option<Document>) -> Self {
        let doc = document.unwrap_or_else(|| Document::new(text.text.clone(), None, None));
        Self {
            tokenized_text: text,
            max_char_buffer,
            sentence_iter: SentenceIterator::new(text, 0).unwrap().peekable(),
            document: doc,
            broken_sentence: false,
        }
    }

    fn tokens_exceed_buffer(&self, token_interval: &TokenInterval) -> bool {
        match get_char_interval(self.tokenized_text, token_interval) {
            Ok(char_interval) => {
                let start = char_interval.start_pos.unwrap_or(0);
                let end = char_interval.end_pos.unwrap_or(0);
                (end - start) > self.max_char_buffer
            }
            Err(_) => false,
        }
    }
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = TextChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let sentence = self.sentence_iter.next()?;
        let mut curr_chunk = TokenInterval {
            start_index: sentence.start_index,
            end_index: sentence.start_index + 1,
        };
        if self.tokens_exceed_buffer(&curr_chunk) {
            self.sentence_iter = SentenceIterator::new(self.tokenized_text, sentence.start_index + 1)
                .unwrap()
                .peekable();
            self.broken_sentence = curr_chunk.end_index < sentence.end_index;
            return Some(TextChunk::new(curr_chunk, Some(self.document.clone())));
        }

        let mut start_of_new_line = None;
        for token_index in curr_chunk.start_index..sentence.end_index {
            if self.tokenized_text.tokens[token_index].first_token_after_newline {
                start_of_new_line = Some(token_index);
            }
            let test_chunk = TokenInterval {
                start_index: curr_chunk.start_index,
                end_index: token_index + 1,
            };
            if self.tokens_exceed_buffer(&test_chunk) {
                if let Some(newline_idx) = start_of_new_line {
                    if newline_idx > 0 {
                        curr_chunk = TokenInterval {
                            start_index: curr_chunk.start_index,
                            end_index: newline_idx,
                        };
                    }
                }
                self.sentence_iter = SentenceIterator::new(self.tokenized_text, curr_chunk.end_index)
                    .unwrap()
                    .peekable();
                self.broken_sentence = true;
                return Some(TextChunk::new(curr_chunk, Some(self.document.clone())));
            } else {
                curr_chunk = test_chunk;
            }
        }

        if self.broken_sentence {
            self.broken_sentence = false;
        } else {
            while let Some(sentence) = self.sentence_iter.peek() {
                let test_chunk = TokenInterval {
                    start_index: curr_chunk.start_index,
                    end_index: sentence.end_index,
                };
                if self.tokens_exceed_buffer(&test_chunk) {
                    self.sentence_iter = SentenceIterator::new(self.tokenized_text, curr_chunk.end_index)
                        .unwrap()
                        .peekable();
                    return Some(TextChunk::new(curr_chunk, Some(self.document.clone())));
                } else {
                    curr_chunk = test_chunk;
                    self.sentence_iter.next();
                }
            }
        }

        Some(TextChunk::new(curr_chunk, Some(self.document.clone())))
    }
}

// ------------------- Tests -------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::{TokenInterval, tokenize};

    #[test]
    fn test_create_token_interval_valid() {
        let interval = create_token_interval(0, 2).unwrap();
        assert_eq!(interval.start_index, 0);
        assert_eq!(interval.end_index, 2);
    }

    #[test]
    #[should_panic]
    fn test_create_token_interval_invalid() {
        create_token_interval(2, 2).unwrap();
    }

    #[test]
    fn test_sanitize() {
        let input = "Hello,\n   world!\tThis is   Rust.";
        let sanitized = sanitize(input).unwrap();
        assert_eq!(sanitized, "Hello, world! This is Rust.");
    }

    #[test]
    fn test_get_token_interval_text() {
        let text = "Hello world!";
        let tokenized_text = tokenize(text);
        let interval = TokenInterval {
            start_index: 0,
            end_index: 2,
        };
        let result = tokens_text(&tokenized_text, &interval).unwrap();
        // The actual result depends on how tokenizer splits the text
        assert!(!result.is_empty());
    }

    #[test]
    fn test_get_char_interval() {
        let text = "Hello world!";
        let tokenized_text = tokenize(text);
        let interval = TokenInterval {
            start_index: 0,
            end_index: 2,
        };
        let char_interval = get_char_interval(&tokenized_text, &interval).unwrap();
        assert!(char_interval.start_pos.is_some());
        assert!(char_interval.end_pos.is_some());
    }

    #[test]
    fn test_sentence_iterator() {
        let text = "Hello world!";
        let tokenized_text = tokenize(text);
        let mut iter = SentenceIterator::new(&tokenized_text, 0).unwrap();
        let first = iter.next().unwrap();
        assert_eq!(first.start_index, 0);
    }

    #[test]
    fn test_chunk_iterator_basic() {
        let text = "Hello world!";
        let tokenized_text = tokenize(text);
        let chunk_iter = ChunkIterator::new(&tokenized_text, 100, None);
        let chunks: Vec<_> = chunk_iter.collect();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_make_batches_of_textchunk() {
        let text = "Hello world!";
        let tokenized_text = tokenize(text);
        let chunk_iter = ChunkIterator::new(&tokenized_text, 100, None);
        let batches = make_batches_of_textchunk(chunk_iter, 1);
        assert!(!batches.is_empty());
    }
}
