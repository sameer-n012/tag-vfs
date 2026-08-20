use crate::data::file_instance::FileInstance;

/**
 * A parsed search query: `file:` and `tag:` terms combined with `and` /
 * `or` and parentheses, e.g. `file:notes.txt and (tag:work or tag:draft)`.
 */
#[derive(Debug, Clone)]
pub enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Term(Field, String),
}

#[derive(Debug, Clone, Copy)]
pub enum Field {
    File,
    Tag,
}

/**
 * Parses a search query. An empty (or whitespace-only) query parses to
 * `Ok(None)`, meaning "no filter". A malformed query returns a short,
 * human-readable error describing what was expected.
 *
 * @param input the raw query text from the search box.
 * @return the parsed expression, or an error message.
 */
pub fn parse(input: &str) -> Result<Option<Expr>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let tokens = tokenize(trimmed);
    let mut pos = 0;
    let expr = parse_or(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(format!("Unexpected \"{}\"", tokens[pos]));
    }
    Ok(Some(expr))
}

/**
 * Checks whether a file matches a parsed query expression. `file:` and
 * `tag:` terms both require an exact (case-insensitive) match, not a
 * substring match.
 *
 * @param expr the parsed query.
 * @param file the file to test.
 * @return true if the file satisfies the query.
 */
pub fn matches(expr: &Expr, file: &FileInstance) -> bool {
    match expr {
        Expr::And(a, b) => matches(a, file) && matches(b, file),
        Expr::Or(a, b) => matches(a, file) || matches(b, file),
        Expr::Term(Field::File, value) => file.name.eq_ignore_ascii_case(value),
        Expr::Term(Field::Tag, value) => {
            file.tags.iter().any(|tag| tag.eq_ignore_ascii_case(value))
        }
    }
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in input.chars() {
        if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else if c == '(' || c == ')' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(c.to_string());
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_or(tokens: &[String], pos: &mut usize) -> Result<Expr, String> {
    let mut left = parse_and(tokens, pos)?;
    while matches!(tokens.get(*pos), Some(t) if t.eq_ignore_ascii_case("or")) {
        *pos += 1;
        let right = parse_and(tokens, pos)?;
        left = Expr::Or(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_and(tokens: &[String], pos: &mut usize) -> Result<Expr, String> {
    let mut left = parse_term(tokens, pos)?;
    while matches!(tokens.get(*pos), Some(t) if t.eq_ignore_ascii_case("and")) {
        *pos += 1;
        let right = parse_term(tokens, pos)?;
        left = Expr::And(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_term(tokens: &[String], pos: &mut usize) -> Result<Expr, String> {
    match tokens.get(*pos).map(|t| t.as_str()) {
        Some("(") => {
            *pos += 1;
            let expr = parse_or(tokens, pos)?;
            match tokens.get(*pos).map(|t| t.as_str()) {
                Some(")") => {
                    *pos += 1;
                    Ok(expr)
                }
                _ => Err("Expected \")\"".to_string()),
            }
        }
        Some(_) => {
            let token = &tokens[*pos];
            *pos += 1;
            parse_field_term(token)
        }
        None => Err("Expected file:value or tag:value".to_string()),
    }
}

fn parse_field_term(token: &str) -> Result<Expr, String> {
    let Some((field, value)) = token.split_once(':') else {
        return Err(format!("Expected file:value or tag:value, got \"{}\"", token));
    };
    if value.is_empty() {
        return Err(format!("Empty value for \"{}:\"", field));
    }
    let field = match field.to_lowercase().as_str() {
        "file" => Field::File,
        "tag" => Field::Tag,
        other => return Err(format!("Unknown field \"{}\" (use file: or tag:)", other)),
    };
    Ok(Expr::Term(field, value.to_string()))
}
