/// Sanitizes JSONC (JSON with Comments and trailing commas) to standard JSON.
pub fn sanitize_jsonc(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < len {
        let c = chars[i];

        if in_string {
            output.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = true;
            output.push(c);
            i += 1;
            continue;
        }

        // Line comment //
        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            i += 2;
            while i < len && chars[i] != '\n' && chars[i] != '\r' {
                i += 1;
            }
            continue;
        }

        // Block comment /* */
        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // skip */
            continue;
        }

        // Trailing comma check
        if c == ',' {
            let mut j = i + 1;
            let mut found_closing = false;
            while j < len {
                let next_c = chars[j];
                if next_c.is_whitespace() {
                    j += 1;
                    continue;
                }
                // Skip comments inside whitespace check
                if next_c == '/' && j + 1 < len && chars[j + 1] == '/' {
                    j += 2;
                    while j < len && chars[j] != '\n' && chars[j] != '\r' {
                        j += 1;
                    }
                    continue;
                }
                if next_c == '/' && j + 1 < len && chars[j + 1] == '*' {
                    j += 2;
                    while j + 1 < len && !(chars[j] == '*' && chars[j + 1] == '/') {
                        j += 1;
                    }
                    j += 2;
                    continue;
                }
                if next_c == '}' || next_c == ']' {
                    found_closing = true;
                }
                break;
            }

            if found_closing {
                i += 1;
                continue;
            }
        }

        output.push(c);
        i += 1;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_comments_and_trailing_commas() {
        let jsonc = r#"{
            // line comment
            "key": "value", /* block comment */
            "arr": [1, 2, ],
        }"#;
        let sanitized = sanitize_jsonc(jsonc);
        let parsed: serde_json::Value = serde_json::from_str(&sanitized).unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["arr"], serde_json::json!([1, 2]));
    }
}
