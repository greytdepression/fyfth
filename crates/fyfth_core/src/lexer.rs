use std::{str::Lines, sync::Arc};

use crate::language::FyfthLanguageExtension;

#[derive(Debug)]
pub struct FyfthLexer<'a> {
    current_line: Option<&'a str>,
    line_byte_index: usize,
    lines: Lines<'a>,
    lang: Arc<FyfthLanguageExtension>,
}

impl<'a> FyfthLexer<'a> {
    pub fn iter(code: &'a str, lang: Arc<FyfthLanguageExtension>) -> Self {
        Self {
            current_line: None,
            line_byte_index: 0,
            lines: code.lines(),
            lang,
        }
    }

    fn get_prefix(&self, ch: char) -> Option<u32> {
        for (index, prefix_ch) in self.lang.prefixes.iter().map(|pi| pi.ch).enumerate() {
            if ch == prefix_ch {
                return Some(index as u32);
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct FyfthWord {
    pub(crate) word: String,
    pub(crate) maybe_prefix: Option<u32>,
    pub(crate) in_quotes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexingState {
    Base,
    Prefixed,
    Word,
    FinishedWord,
    QuoteStart,
    QuotedWord,
    FinishedQuotedWord,
}

impl<'a> Iterator for FyfthLexer<'a> {
    type Item = Result<FyfthWord, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_line.is_none()
            || self.current_line.unwrap().split('#').next().unwrap()[self.line_byte_index..]
                .trim()
                .is_empty()
        {
            self.current_line = Some(self.lines.next()?);
            self.line_byte_index = 0;

            // if the line is empty (after removing comments), try to get the next line
            if self
                .current_line
                .unwrap()
                .split('#')
                .next()
                .unwrap()
                .trim()
                .is_empty()
            {
                self.current_line = None;
            }
        }

        let current_line =
            &self.current_line.unwrap().split('#').next().unwrap()[self.line_byte_index..];

        let mut state = LexingState::Base;

        let mut start_index = 0;
        let mut end_index = 0;
        let mut escaped = false;

        let mut maybe_prefix = None;

        use LexingState::*;
        for (index, ch) in current_line.char_indices() {
            match (state, ch) {
                (Base, '"') => state = QuoteStart,
                (Base, _) if ch.is_whitespace() => {}
                (Base, _) => {
                    if let Some(prefix) = self.get_prefix(ch) {
                        state = Prefixed;
                        maybe_prefix = Some(prefix);
                    } else {
                        start_index = index;
                        state = Word;
                    }
                }
                (Prefixed, '"') => state = QuoteStart,
                (Prefixed, _) if ch.is_whitespace() => {}
                (Prefixed, _) => {
                    start_index = index;
                    state = Word;
                }
                (Word, _) if ch.is_whitespace() => {
                    end_index = index;
                    state = FinishedWord;
                    break;
                }
                (Word, _) => {}
                // empty string
                (QuoteStart, '"') => {
                    start_index = index;
                    end_index = index + 1;
                    state = FinishedQuotedWord;
                    break;
                }
                (QuoteStart, _) => {
                    start_index = index;
                    state = QuotedWord;
                }
                (QuotedWord, '\\') => {
                    escaped = !escaped;
                }
                (QuotedWord, '"') if !escaped => {
                    end_index = index;
                    state = FinishedQuotedWord;
                    break;
                }
                (QuotedWord, _) => escaped = false,
                (FinishedWord, _) | (FinishedQuotedWord, _) => unreachable!(),
            }
        }

        // Finish off unfinished words
        match state {
            Base => panic!("It should not be possible to end up in the base state"),
            Prefixed => return Some(Err(())),
            Word => {
                end_index = current_line.len();
                state = FinishedWord;
            }
            QuoteStart | QuotedWord => {
                end_index = current_line.len();
                state = FinishedQuotedWord;
            }
            FinishedWord => {}
            FinishedQuotedWord => {
                // add an additonal `1` to `self.line_byte_index` to skip the closing `"`
                self.line_byte_index += 1;
            }
        }

        self.line_byte_index += end_index;

        match state {
            FinishedWord => Some(Ok(FyfthWord {
                word: current_line[start_index..end_index].to_string(),
                maybe_prefix,
                in_quotes: false,
            })),
            FinishedQuotedWord => {
                let mut processed_word = String::with_capacity(end_index - start_index);
                let mut escaped = false;
                for ch in current_line[start_index..end_index].chars() {
                    if !escaped {
                        match ch {
                            '\\' => escaped = true,
                            _ => processed_word.push(ch),
                        }
                    } else {
                        match ch {
                            'n' => processed_word.push('\n'),
                            '"' => processed_word.push('"'),
                            't' => processed_word.push('\t'),
                            'r' => processed_word.push('\r'),
                            '\\' => processed_word.push('\\'),
                            _ => eprintln!(
                                "Illegal escape code `\\{ch}` in {}",
                                self.current_line.unwrap()
                            ),
                        }
                        escaped = false;
                    }
                }
                Some(Ok(FyfthWord {
                    word: processed_word,
                    maybe_prefix,
                    in_quotes: true,
                }))
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{interpreter::FyfthVariant, language::FyfthLanguageExtension};

    use super::FyfthLexer;

    #[test]
    fn test_lexer_simple() {
        let input = r#"foo bar baz"#;
        let lang = FyfthLanguageExtension::base_fyfth();
        let lexer = FyfthLexer::iter(input, Arc::new(lang));
        let commands: Vec<_> = lexer.map(|fw| fw.unwrap().word).collect();
        let expected = &["foo", "bar", "baz"];

        assert_eq!(&commands, expected);
    }

    #[test]
    fn test_lexer_multi_line() {
        let input = r#"foo
bar
baz
"#;
        let lang = FyfthLanguageExtension::base_fyfth();
        let lexer = FyfthLexer::iter(input, Arc::new(lang));
        let commands: Vec<_> = lexer.map(|fw| fw.unwrap().word).collect();
        let expected = &["foo", "bar", "baz"];

        assert_eq!(&commands, expected);
    }

    #[test]
    fn test_lexer_multi_line_with_comments() {
        let input = r#"foo          # this is the first command
bar                                       # and this is the second
baz                                       # and third
"#;
        let lang = FyfthLanguageExtension::base_fyfth();
        let lexer = FyfthLexer::iter(input, Arc::new(lang));
        let commands: Vec<_> = lexer.map(|fw| fw.unwrap().word).collect();
        let expected = &["foo", "bar", "baz"];

        assert_eq!(&commands, expected);
    }

    #[test]
    fn test_lexer_multi_line_with_comments_quotes() {
        let input = r#""foo"        # this is the first command
bar                                       # and this is the second
"baz"                                     # and third
"#;
        let lang = FyfthLanguageExtension::base_fyfth();
        let lexer = FyfthLexer::iter(input, Arc::new(lang));
        let commands: Vec<_> = lexer.map(|fw| fw.unwrap().word).collect();
        let expected = &["foo", "bar", "baz"];

        assert_eq!(&commands, expected);
    }

    #[test]
    fn test_lexer_multi_line_with_comments_quotes_missing_closing_quote() {
        let input = r#""foo         # this is the first command
bar                                       # and this is the second
"baz                                      # and third
"#;
        let lang = FyfthLanguageExtension::base_fyfth();
        let lexer = FyfthLexer::iter(input, Arc::new(lang));
        let commands: Vec<_> = lexer.map(|fw| fw.unwrap().word).collect();
        let expected = &[
            "foo         ",
            "bar",
            "baz                                      ",
        ];

        assert_eq!(&commands, expected);
    }

    #[test]
    fn test_lexer_multi_line_with_comments_quotes_escaped_chars() {
        let input = r#""foo\nand or\nbar"      # this is the first command
bar                                                  # and this is the second
"baz\\\\\\"                                          # and third
"#;
        let lang = FyfthLanguageExtension::base_fyfth();
        let lexer = FyfthLexer::iter(input, Arc::new(lang));
        let commands: Vec<_> = lexer.map(|fw| fw.unwrap().word).collect();
        let expected = &["foo\nand or\nbar", "bar", "baz\\\\\\"];

        assert_eq!(&commands, expected);
    }

    fn debug_prefix_parser_fn(
        _word: &str,
        _lang: &FyfthLanguageExtension,
    ) -> Result<Vec<FyfthVariant>, ()> {
        Ok(vec![])
    }

    #[test]
    fn test_lexer_prefix() {
        let input = r#"@foo bar ^"foo bar" baz"#;
        let mut lang = FyfthLanguageExtension::base_fyfth();
        lang.with_prefix('@', debug_prefix_parser_fn);
        lang.with_prefix('^', debug_prefix_parser_fn);
        let lexer: Vec<_> = FyfthLexer::iter(input, Arc::new(lang)).collect();
        let command_words: Vec<_> = lexer
            .iter()
            .map(|fw| fw.as_ref().unwrap().word.clone())
            .collect();
        let command_prefixes: Vec<_> = lexer
            .iter()
            .map(|fw| fw.as_ref().unwrap().maybe_prefix)
            .collect();
        let expected_words = &["foo", "bar", "foo bar", "baz"];
        let expected_prefixes = &[Some(0), None, Some(1), None];

        assert_eq!(&command_words, expected_words);
        assert_eq!(&command_prefixes, expected_prefixes);
    }
}
