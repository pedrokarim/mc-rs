use std::fmt;

/// Parsed command line ready for registry dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommandLine {
    pub original: String,
    pub label: String,
    pub args: Vec<String>,
    pub raw_args: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineParseError {
    Empty,
    UnterminatedQuote,
    TrailingEscape,
}

impl fmt::Display for CommandLineParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandLineParseError::Empty => write!(f, "Empty command line"),
            CommandLineParseError::UnterminatedQuote => write!(f, "Unterminated quoted string"),
            CommandLineParseError::TrailingEscape => write!(f, "Trailing escape in command line"),
        }
    }
}

/// Split a command line like PocketMine's quote-aware parser.
pub fn tokenize_command_line(input: &str) -> Result<Vec<String>, CommandLineParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CommandLineParseError::Empty);
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;
    let mut escape = false;

    for ch in trimmed.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' => escape = true,
            '"' | '\'' => {
                if let Some(active_quote) = quote {
                    if active_quote == ch {
                        quote = None;
                    } else {
                        current.push(ch);
                    }
                } else {
                    quote = Some(ch);
                }
            }
            c if c.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if escape {
        return Err(CommandLineParseError::TrailingEscape);
    }
    if quote.is_some() {
        return Err(CommandLineParseError::UnterminatedQuote);
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    if tokens.is_empty() {
        return Err(CommandLineParseError::Empty);
    }
    Ok(tokens)
}

pub fn parse_command_line(input: &str) -> Result<ParsedCommandLine, CommandLineParseError> {
    let trimmed = input.trim();
    let tokens = tokenize_command_line(trimmed)?;
    let label = tokens[0].trim_start_matches('/').to_ascii_lowercase();

    let raw_args = trimmed
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim_start().to_string())
        .unwrap_or_default();

    Ok(ParsedCommandLine {
        original: trimmed.to_string(),
        label,
        args: tokens.into_iter().skip(1).collect(),
        raw_args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_quotes_and_escapes() {
        let tokens =
            tokenize_command_line(r#"/say "hello world" 'two words' plain\ value"#).unwrap();
        assert_eq!(
            tokens,
            vec![
                "/say".to_string(),
                "hello world".to_string(),
                "two words".to_string(),
                "plain value".to_string(),
            ]
        );
    }

    #[test]
    fn parse_command_line_normalizes_slash_and_rest() {
        let parsed = parse_command_line(r#"  /tell Steve "hi there"  "#).unwrap();
        assert_eq!(parsed.label, "tell");
        assert_eq!(
            parsed.args,
            vec!["Steve".to_string(), "hi there".to_string()]
        );
        assert_eq!(parsed.raw_args, r#"Steve "hi there""#);
    }

    #[test]
    fn parse_command_line_rejects_unterminated_quotes() {
        assert_eq!(
            parse_command_line(r#"/say "oops"#).unwrap_err(),
            CommandLineParseError::UnterminatedQuote
        );
    }
}
