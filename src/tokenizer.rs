use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TokenizerError {
    #[error("Invalid token interval. start_index={start_index}, end_index={end_index}, total_tokens={total_tokens}.")]
    InvalidTokenInterval {
        start_index: usize,
        end_index: usize,
        total_tokens: usize,
    },

    #[error("SentenceRangeError: start_token_index={start_token_index} out of range. Total tokens: {total_tokens}.")]
    SentenceRangeError {
        start_token_index: usize,
        total_tokens: usize,
    },
}

#[derive(Debug, Clone)]
pub struct CharInterval {
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, Clone)]
pub struct TokenInterval {
    pub start_index: usize,
    pub end_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Word,
    Number,
    Punctuation,
    Acronym,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub index: usize,
    pub token_type: TokenType,
    pub char_interval: CharInterval,
    pub first_token_after_newline: bool,
}

#[derive(Debug, Clone)]
pub struct TokenizedText {
    pub text: String,
    pub tokens: Vec<Token>,
}

lazy_static! {
    static ref DIGITS_REGEX: Regex = Regex::new(r"[0-9]+").unwrap();
    static ref SLASH_ABBREV_REGEX: Regex = Regex::new(r"[A-Za-z0-9]+(?:/[A-Za-z0-9]+)+").unwrap();
    static ref END_OF_SENTENCE_REGEX: Regex = Regex::new(r"[.?!。？！]$").unwrap();
    static ref TOKEN_REGEX: Regex =
        Regex::new(r"[A-Za-z0-9]+(?:/[A-Za-z0-9]+)+|[\u4e00-\u9fff]+|[A-Za-z]+|[0-9]+|[^\u4e00-\u9fffA-Za-z0-9\s]+")
            .unwrap();
    static ref WORD_REGEX: Regex = Regex::new(r"(?:[\u4e00-\u9fff]+|[A-Za-z]+|[0-9]+)\b").unwrap();
    static ref CHINESE_REGEX: Regex = Regex::new(r"[\u4e00-\u9fff]+").unwrap();
    static ref KNOWN_ABBREVIATIONS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("Mr.");
        set.insert("Mrs.");
        set.insert("Ms.");
        set.insert("Dr.");
        set.insert("Prof.");
        set.insert("St.");
        set
    };
}

/// Tokenize text into TokenizedText
pub fn tokenize(text: &str) -> TokenizedText {
    let mut tokenized = TokenizedText {
        text: text.to_string(),
        tokens: Vec::new(),
    };

    let mut previous_end = 0;

    for (token_index, mat) in TOKEN_REGEX.find_iter(text).enumerate() {
        let start_pos = mat.start();
        let end_pos = mat.end();
        let matched_text = mat.as_str();

        let mut token = Token {
            index: token_index,
            char_interval: CharInterval { start_pos, end_pos },
            token_type: TokenType::Word,
            first_token_after_newline: false,
        };

        // Check newline before token
        if token_index > 0 {
            let gap = &text[previous_end..start_pos];
            if gap.contains('\n') || gap.contains('\r') {
                token.first_token_after_newline = true;
            }
        }

        // Classify token type
        if DIGITS_REGEX.is_match(matched_text) {
            token.token_type = TokenType::Number;
        } else if SLASH_ABBREV_REGEX.is_match(matched_text) {
            token.token_type = TokenType::Acronym;
        } else if CHINESE_REGEX.is_match(matched_text) {
            token.token_type = TokenType::Word;
        } else if WORD_REGEX.is_match(matched_text) {
            token.token_type = TokenType::Word;
        } else {
            token.token_type = TokenType::Punctuation;
        }

        tokenized.tokens.push(token);
        previous_end = end_pos;
    }

    tokenized
}

/// Reconstruct substring from token interval
pub fn tokens_text(tokenized_text: &TokenizedText, token_interval: &TokenInterval) -> Result<String, TokenizerError> {
    if token_interval.start_index >= token_interval.end_index || token_interval.end_index > tokenized_text.tokens.len()
    {
        return Err(TokenizerError::InvalidTokenInterval {
            start_index: token_interval.start_index,
            end_index: token_interval.end_index,
            total_tokens: tokenized_text.tokens.len(),
        });
    }

    let start_token = &tokenized_text.tokens[token_interval.start_index];
    let end_token = &tokenized_text.tokens[token_interval.end_index - 1];

    Ok(tokenized_text.text[start_token.char_interval.start_pos..end_token.char_interval.end_pos].to_string())
}

/// Determine if token is end of sentence
fn is_end_of_sentence_token(text: &str, tokens: &[Token], current_idx: usize) -> bool {
    let token_text = &text[tokens[current_idx].char_interval.start_pos..tokens[current_idx].char_interval.end_pos];

    if END_OF_SENTENCE_REGEX.is_match(token_text) {
        if current_idx > 0 {
            let prev_token_text =
                &text[tokens[current_idx - 1].char_interval.start_pos..tokens[current_idx - 1].char_interval.end_pos];
            let combined = format!("{}{}", prev_token_text, token_text);
            if KNOWN_ABBREVIATIONS.contains(combined.as_str()) {
                return false;
            }
        }
        return true;
    }
    false
}

/// Heuristic: newline + uppercase = sentence boundary
fn is_sentence_break_after_newline(text: &str, tokens: &[Token], current_idx: usize) -> bool {
    if current_idx + 1 >= tokens.len() {
        return false;
    }

    let gap_text = &text[tokens[current_idx].char_interval.end_pos..tokens[current_idx + 1].char_interval.start_pos];

    if !gap_text.contains('\n') {
        return false;
    }

    let next_token_text =
        &text[tokens[current_idx + 1].char_interval.start_pos..tokens[current_idx + 1].char_interval.end_pos];
    !next_token_text.is_empty() && next_token_text.chars().next().unwrap().is_uppercase()
}

/// Find sentence range
pub fn find_sentence_range(
    text: &str,
    tokens: &[Token],
    start_token_index: usize,
) -> Result<TokenInterval, TokenizerError> {
    if start_token_index >= tokens.len() {
        return Err(TokenizerError::SentenceRangeError {
            start_token_index,
            total_tokens: tokens.len(),
        });
    }

    let mut i = start_token_index;
    while i < tokens.len() {
        if tokens[i].token_type == TokenType::Punctuation && is_end_of_sentence_token(text, tokens, i) {
            return Ok(TokenInterval {
                start_index: start_token_index,
                end_index: i + 1,
            });
        }

        if is_sentence_break_after_newline(text, tokens, i) {
            return Ok(TokenInterval {
                start_index: start_token_index,
                end_index: i + 1,
            });
        }

        i += 1;
    }

    Ok(TokenInterval {
        start_index: start_token_index,
        end_index: tokens.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let text = "Dr. Smith went to Paris.";
        let tokenized = tokenize(text);

        assert!(!tokenized.tokens.is_empty());
        assert_eq!(
            &text[tokenized.tokens[0].char_interval.start_pos..tokenized.tokens[0].char_interval.end_pos],
            "Dr"
        );
        assert_eq!(tokenized.tokens[0].token_type, TokenType::Word);
    }

    #[test]
    fn test_tokens_text_reconstruct() {
        let text = "Hello world!";
        let tokenized = tokenize(text);

        let interval = TokenInterval {
            start_index: 0,
            end_index: tokenized.tokens.len() - 1,
        };
        let reconstructed = tokens_text(&tokenized, &interval).unwrap();
        assert_eq!(reconstructed, "Hello world");
    }

    #[test]
    fn test_find_sentence_range() {
        let text = "Hello world! This is Rust.";
        let tokenized = tokenize(text);

        let range1 = find_sentence_range(&tokenized.text, &tokenized.tokens, 0).unwrap();
        let sentence1 = tokens_text(&tokenized, &range1).unwrap();
        assert_eq!(sentence1, "Hello world!");

        let range2 = find_sentence_range(&tokenized.text, &tokenized.tokens, range1.end_index).unwrap();
        let sentence2 = tokens_text(&tokenized, &range2).unwrap();
        assert_eq!(sentence2, "This is Rust.");
    }

    #[test]
    fn test_invalid_token_interval() {
        let text = "Hello world!";
        let tokenized = tokenize(text);

        let invalid_interval = TokenInterval {
            start_index: 5,
            end_index: 2,
        };

        let result = tokens_text(&tokenized, &invalid_interval);
        assert!(result.is_err());
    }

    #[test]
    fn test_chinese_tokenization() {
        let text = "宝玉今日穿了一件月白缎子袍子，腰系丝绦，头戴紫金冠。";
        let tokenized = tokenize(text);

        assert!(!tokenized.tokens.is_empty());

        // Check that Chinese characters are tokenized as words
        let mut found_chinese_word = false;
        for token in &tokenized.tokens {
            let token_text = &text[token.char_interval.start_pos..token.char_interval.end_pos];
            if token.token_type == TokenType::Word && token_text.chars().any(|c| c >= '\u{4e00}' && c <= '\u{9fff}') {
                found_chinese_word = true;
                break;
            }
        }
        assert!(found_chinese_word, "Should find at least one Chinese word token");

        // Check that punctuation is properly separated
        let punctuation_tokens: Vec<_> = tokenized
            .tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Punctuation)
            .collect();
        assert!(!punctuation_tokens.is_empty(), "Should find punctuation tokens");
    }

    #[test]
    fn test_mixed_chinese_english_tokenization() {
        let text = "Hello世界! This is测试.";
        let tokenized = tokenize(text);

        assert!(!tokenized.tokens.is_empty());

        // Should have both English words and Chinese words
        let has_english = tokenized.tokens.iter().any(|t| {
            let token_text = &text[t.char_interval.start_pos..t.char_interval.end_pos];
            t.token_type == TokenType::Word && token_text.chars().all(|c| c.is_ascii_alphabetic())
        });

        let has_chinese = tokenized.tokens.iter().any(|t| {
            let token_text = &text[t.char_interval.start_pos..t.char_interval.end_pos];
            t.token_type == TokenType::Word && token_text.chars().any(|c| c >= '\u{4e00}' && c <= '\u{9fff}')
        });

        assert!(has_english, "Should find English words");
        assert!(has_chinese, "Should find Chinese words");
    }
}
