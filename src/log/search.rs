use super::Event;
#[derive(Debug)]
pub struct Term {
    pub field: Option<String>,
    pub op: String,
    pub value: String,
}
pub fn search_terms(query: &str) -> Result<Vec<Term>, String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    for c in query.chars() {
        if let Some(q) = quote {
            token.push(c);
            if c == q {
                quote = None;
            }
        } else if (c == '"' || c == '\'')
            && (token.is_empty() || token.ends_with([':', '=', '>', '<']))
        {
            quote = Some(c);
            token.push(c);
        } else if c.is_whitespace() {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
        } else {
            token.push(c);
        }
    }
    if quote.is_some() {
        return Err("Close the quoted search value.".into());
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
        .into_iter()
        .map(|token| {
            let split = if token.starts_with(['"', '\'']) {
                None
            } else {
                token.find([':', '=', '>', '<'])
            };
            if let Some(i) = split {
                let rest = &token[i..];
                let len = if rest.starts_with(">=") || rest.starts_with("<=") {
                    2
                } else {
                    1
                };
                let value = token[i + len..].trim_matches(['"', '\'']).to_owned();
                if value.is_empty() {
                    return Err(format!("Enter a value after {}.", &token[..i + len]));
                }
                if matches!(&rest[..len], ">" | "<" | ">=" | "<=")
                    && value.parse::<f64>().ok().is_none_or(|n| !n.is_finite())
                {
                    return Err(
                        "Numeric comparisons require a number; durations use milliseconds.".into(),
                    );
                }
                Ok(Term {
                    field: Some(token[..i].into()),
                    op: rest[..len].into(),
                    value,
                })
            } else {
                let quoted = token.starts_with(['"', '\'']);
                Ok(Term {
                    field: None,
                    op: if quoted { "contains" } else { "fuzzy" }.into(),
                    value: token.trim_matches(['"', '\'']).into(),
                })
            }
        })
        .collect()
}
pub fn matches(event: &Event, terms: &[Term]) -> bool {
    terms.iter().all(|t| {
        let Some(field) = &t.field else {
            let hay = &event.projection().search;
            let needle = t.value.to_lowercase();
            if t.op == "contains" || hay.contains(&needle) {
                return hay.contains(&needle);
            }
            let mut chars = needle.chars();
            let mut next = chars.next();
            for c in hay.chars() {
                if next == Some(c) {
                    next = chars.next();
                    if next.is_none() {
                        return true;
                    }
                }
            }
            return next.is_none();
        };
        let actual = match field.as_str() {
            "file" => event.file().to_owned(),
            "level" => event.level().to_owned(),
            "duration" => event.duration().map(|n| n.to_string()).unwrap_or_default(),
            "status" => event.status().map(|n| n.to_string()).unwrap_or_default(),
            _ => event.field(field),
        };
        if actual.is_empty() {
            return false;
        }
        match t.op.as_str() {
            ">" | "<" | ">=" | "<=" => {
                let (Ok(a), Ok(b)) = (actual.parse::<f64>(), t.value.parse::<f64>()) else {
                    return false;
                };
                match t.op.as_str() {
                    ">" => a > b,
                    "<" => a < b,
                    ">=" => a >= b,
                    _ => a <= b,
                }
            }
            _ => actual.eq_ignore_ascii_case(&t.value),
        }
    })
}
