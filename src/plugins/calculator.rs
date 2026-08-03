use crate::plugin::{ExecutionResult, LauncherPlugin, SearchResult};
use evalexpr::{eval, Value};
use gtk::gdk;
use gtk::prelude::*;
use std::process::Command;

pub struct CalculatorPlugin;

impl CalculatorPlugin {
    pub fn new() -> Self {
        Self
    }
}

fn strip_keywords(query: &str) -> String {
    let mut q = query.to_lowercase();
    let keywords = [
        "mitternachtsformel",
        "mitternachts formel",
        "quadratic formula",
        "abc-formel",
        "abc formel",
        "pq-formel",
        "pq formel",
        "mitternacht",
        "quadratic",
        "abc",
        "pq",
    ];
    for kw in &keywords {
        q = q.replace(kw, " ");
    }
    q
}

/// Helper to parse 'a', 'b', and 'c' coefficients for quadratic formula (abc-formula)
fn parse_abc(query: &str) -> Option<(f64, f64, f64)> {
    let clean_q = strip_keywords(query);
    
    let mut a: Option<f64> = None;
    let mut b: Option<f64> = None;
    let mut c: Option<f64> = None;

    let chars: Vec<char> = clean_q.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == 'a' || ch == 'b' || ch == 'c' {
            let mut scan_idx = i + 1;
            while scan_idx < chars.len() && (chars[scan_idx] == '=' || chars[scan_idx] == ':' || chars[scan_idx].is_whitespace()) {
                scan_idx += 1;
            }
            
            let mut end_idx = scan_idx;
            if end_idx < chars.len() && (chars[end_idx] == '-' || chars[end_idx] == '+') {
                end_idx += 1;
                while end_idx < chars.len() && chars[end_idx].is_whitespace() {
                    end_idx += 1;
                }
            }
            
            let start_digits = end_idx;
            while end_idx < chars.len() && (chars[end_idx].is_ascii_digit() || chars[end_idx] == '.') {
                end_idx += 1;
            }
            
            if end_idx > start_digits {
                let sign = if scan_idx < chars.len() && chars[scan_idx] == '-' { "-" } else { "" };
                let digits: String = chars[start_digits..end_idx].iter().collect();
                let num_str = format!("{}{}", sign, digits);
                if let Ok(val) = num_str.parse::<f64>() {
                    match ch {
                        'a' => a = Some(val),
                        'b' => b = Some(val),
                        'c' => c = Some(val),
                        _ => {}
                    }
                }
                if end_idx > 0 {
                    i = end_idx - 1;
                }
            }
        }
        i += 1;
    }

    if let (Some(av), Some(bv), Some(cv)) = (a, b, c) {
        Some((av, bv, cv))
    } else {
        None
    }
}

/// Helper to parse 'p' and 'q' coefficients for the pq-formula
fn parse_pq(query: &str) -> Option<(f64, f64)> {
    let clean_q = strip_keywords(query);

    let mut p: Option<f64> = None;
    let mut q_val: Option<f64> = None;

    let chars: Vec<char> = clean_q.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == 'p' || ch == 'q' {
            let mut scan_idx = i + 1;
            while scan_idx < chars.len() && (chars[scan_idx] == '=' || chars[scan_idx] == ':' || chars[scan_idx].is_whitespace()) {
                scan_idx += 1;
            }
            
            let mut end_idx = scan_idx;
            if end_idx < chars.len() && (chars[end_idx] == '-' || chars[end_idx] == '+') {
                end_idx += 1;
                while end_idx < chars.len() && chars[end_idx].is_whitespace() {
                    end_idx += 1;
                }
            }
            
            let start_digits = end_idx;
            while end_idx < chars.len() && (chars[end_idx].is_ascii_digit() || chars[end_idx] == '.') {
                end_idx += 1;
            }
            
            if end_idx > start_digits {
                let sign = if scan_idx < chars.len() && chars[scan_idx] == '-' { "-" } else { "" };
                let digits: String = chars[start_digits..end_idx].iter().collect();
                let num_str = format!("{}{}", sign, digits);
                if let Ok(val) = num_str.parse::<f64>() {
                    match ch {
                        'p' => p = Some(val),
                        'q' => q_val = Some(val),
                        _ => {}
                    }
                }
                if end_idx > 0 {
                    i = end_idx - 1;
                }
            }
        }
        i += 1;
    }

    if let (Some(pv), Some(qv)) = (p, q_val) {
        Some((pv, qv))
    } else {
        None
    }
}

/// Pre-processes mathematical expressions from German/English words into standard math syntax
fn translate_math_expression(expr: &str) -> String {
    let mut s = expr.to_lowercase();
    
    // 1. Core constants and keywords translations
    s = s.replace("kreiszahl", "3.141592653589793");
    s = s.replace("pi", "3.141592653589793");
    s = s.replace("eulersche zahl", "2.718281828459045");
    s = s.replace("euler", "2.718281828459045");
    s = s.replace("quadratwurzel", "sqrt");
    s = s.replace("wurzel", "sqrt");
    s = s.replace("square root", "sqrt");
    s = s.replace("sinus", "sin");
    s = s.replace("cosinus", "cos");
    s = s.replace("tangens", "tan");
    s = s.replace("geteilt durch", "/");
    s = s.replace("durch", "/");
    s = s.replace("div", "/");
    s = s.replace("mal", "*");
    s = s.replace("plus", "+");
    s = s.replace("und", "+");
    s = s.replace("minus", "-");
    s = s.replace("weniger", "-");
    s = s.replace("hoch", "^");
    s = s.replace("power", "^");

    // 2. Wrap function calls in math:: prefix
    let funcs = ["sqrt", "sin", "cos", "tan", "ln", "log"];
    for func in &funcs {
        let mut search_idx = 0;
        while let Some(pos) = s[search_idx..].find(func) {
            let idx = search_idx + pos;
            if idx > 0 {
                let prev_char = s.as_bytes()[idx - 1] as char;
                if prev_char.is_ascii_alphabetic() || prev_char == ':' {
                    search_idx = idx + func.len();
                    continue;
                }
            }
            
            let mut scan_idx = idx + func.len();
            let bytes = s.as_bytes();
            while scan_idx < bytes.len() && (bytes[scan_idx] as char).is_whitespace() {
                scan_idx += 1;
            }
            
            if scan_idx >= bytes.len() {
                search_idx = idx + func.len();
                continue;
            }
            
            if bytes[scan_idx] as char == '(' {
                let mut paren_count = 1;
                let mut end_idx = scan_idx + 1;
                while end_idx < bytes.len() && paren_count > 0 {
                    if bytes[end_idx] as char == '(' {
                        paren_count += 1;
                    } else if bytes[end_idx] as char == ')' {
                        paren_count -= 1;
                    }
                    end_idx += 1;
                }
                
                if paren_count == 0 {
                    let inner = &s[scan_idx + 1..end_idx - 1];
                    let normalized_inner = translate_math_expression(inner);
                    let replacement = format!("math::{}({})", func, normalized_inner);
                    s.replace_range(idx..end_idx, &replacement);
                    search_idx = idx + replacement.len();
                } else {
                    search_idx = idx + func.len();
                }
            } else {
                let mut end_idx = scan_idx;
                while end_idx < bytes.len() {
                    let c = bytes[end_idx] as char;
                    if c.is_ascii_alphanumeric() || c == '.' {
                        end_idx += 1;
                    } else {
                        break;
                    }
                }
                
                if end_idx > scan_idx {
                    let arg = &s[scan_idx..end_idx];
                    let normalized_arg = translate_math_expression(arg);
                    let replacement = format!("math::{}({})", func, normalized_arg);
                    s.replace_range(idx..end_idx, &replacement);
                    search_idx = idx + replacement.len();
                } else {
                    search_idx = idx + func.len();
                }
            }
        }
    }
    s
}

fn convert_integers_to_floats(expr: &str) -> String {
    let mut result = String::new();
    let mut current_number = String::new();
    
    for c in expr.chars() {
        if c.is_ascii_digit() {
            current_number.push(c);
        } else if c == '.' {
            current_number.push(c);
        } else {
            if !current_number.is_empty() {
                if !current_number.contains('.') {
                    current_number.push_str(".0");
                }
                result.push_str(&current_number);
                current_number.clear();
            }
            result.push(c);
        }
    }
    
    if !current_number.is_empty() {
        if !current_number.contains('.') {
            current_number.push_str(".0");
        }
        result.push_str(&current_number);
    }
    
    result
}

fn format_float(val: f64) -> String {
    let rounded = (val * 100.0).round() / 100.0;
    if rounded == rounded.trunc() {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{:.2}", rounded);
        if s.ends_with('0') {
            s[..s.len() - 1].to_string()
        } else {
            s
        }
    }
}

impl LauncherPlugin for CalculatorPlugin {
    fn id(&self) -> &str {
        "calculator"
    }

    fn accepts(&self, query: &str) -> bool {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return false;
        }

        if parse_abc(trimmed).is_some() {
            return true;
        }

        if parse_pq(trimmed).is_some() {
            return true;
        }

        let translated = translate_math_expression(trimmed);
        let float_expr = convert_integers_to_floats(&translated);

        let has_operators = float_expr.chars().any(|c| {
            c == '+' || c == '*' || c == '/' || c == '^' || c == '%' || c == '<' || c == '>' || c == '=' || c == '(' || (c == '-' && float_expr.len() > 1)
        });

        if !has_operators {
            return false;
        }

        eval(&float_expr).is_ok()
    }

    fn query(&self, query: &str) -> Vec<SearchResult> {
        let trimmed = query.trim();

        // 1. Quadratic Equation solver (abc-formel / mitternachtsformel)
        if let Some((a, b, c)) = parse_abc(trimmed) {
            if a == 0.0 {
                return vec![SearchResult {
                    id: "calc:error".to_string(),
                    title: "Error: a cannot be 0".to_string(),
                    description: Some("abc-Formula requires a != 0".to_string()),
                    icon: Some("accessories-calculator".to_string()),
                    score: 1000,
                    last_used: None,
                }];
            }

            let d = b * b - 4.0 * a * c;
            let title = if d > 0.0 {
                let r_d = d.sqrt();
                let x1 = (-b + r_d) / (2.0 * a);
                let x2 = (-b - r_d) / (2.0 * a);
                format!("x1 = {}, x2 = {}", format_float(x1), format_float(x2))
            } else if d == 0.0 {
                let x = -b / (2.0 * a);
                format!("x = {}", format_float(x))
            } else {
                let real = -b / (2.0 * a);
                let imag = (-d).sqrt() / (2.0 * a);
                format!("x1 = {} + {}i, x2 = {} - {}i", format_float(real), format_float(imag), format_float(real), format_float(imag))
            };

            return vec![SearchResult {
                id: format!("calc:{}", title),
                title: title.clone(),
                description: Some(format!("abc-Formula: a={}, b={}, c={} (D={:.2})", a, b, c, d)),
                icon: Some("accessories-calculator".to_string()),
                score: 1000,
                last_used: None,
            }];
        }

        // 2. pq-Formula solver
        if let Some((p, q_val)) = parse_pq(trimmed) {
            let d = (p / 2.0) * (p / 2.0) - q_val;
            let title = if d > 0.0 {
                let r_d = d.sqrt();
                let x1 = -p / 2.0 + r_d;
                let x2 = -p / 2.0 - r_d;
                format!("x1 = {}, x2 = {}", format_float(x1), format_float(x2))
            } else if d == 0.0 {
                let x = -p / 2.0;
                format!("x = {}", format_float(x))
            } else {
                let real = -p / 2.0;
                let imag = (-d).sqrt();
                format!("x1 = {} + {}i, x2 = {} - {}i", format_float(real), format_float(imag), format_float(real), format_float(imag))
            };

            return vec![SearchResult {
                id: format!("calc:{}", title),
                title: title.clone(),
                description: Some(format!("pq-Formula: p={}, q={} (D={:.2})", p, q_val, d)),
                icon: Some("accessories-calculator".to_string()),
                score: 1000,
                last_used: None,
            }];
        }

        // 3. Standard expression evaluation
        if !self.accepts(trimmed) {
            return Vec::new();
        }

        let translated = translate_math_expression(trimmed);
        let float_expr = convert_integers_to_floats(&translated);

        match eval(&float_expr) {
            Ok(value) => {
                let display_val = match &value {
                    Value::Float(f) => format_float(*f),
                    Value::Int(i) => format_float(*i as f64),
                    Value::Boolean(b) => format!("{}", b),
                    Value::String(s) => s.clone(),
                    _ => format!("{:?}", value),
                };

                vec![SearchResult {
                    id: format!("calc:{}", display_val),
                    title: display_val.clone(),
                    description: Some(format!("Calculator: {} = {}", trimmed, display_val)),
                    icon: Some("accessories-calculator".to_string()),
                    score: 1000,
                    last_used: None,
                }]
            }
            Err(_) => Vec::new(),
        }
    }

    fn execute(&self, result_id: &str, _shift_pressed: bool) -> ExecutionResult {
        if let Some(result_val) = result_id.strip_prefix("calc:") {
            let child = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn();
            
            let mut success = false;
            if let Ok(mut child) = child {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    if stdin.write_all(result_val.as_bytes()).is_ok() {
                        drop(stdin);
                        if let Ok(status) = child.wait() {
                            if status.success() {
                                success = true;
                            }
                        }
                    }
                }
            }

            if !success {
                if let Some(display) = gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(result_val);
                }
            }
            ExecutionResult::CloseLauncher
        } else {
            ExecutionResult::Error("Invalid calculator action ID".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_accepts() {
        let calc = CalculatorPlugin::new();
        assert!(calc.accepts("2 + 2"));
        assert!(calc.accepts("10 * (5 - 3)"));
        assert!(calc.accepts("wurzel 16"));
        assert!(calc.accepts("mitternachtsformel a 10 b 20 c -5"));
        assert!(calc.accepts("pq p 4 q 3"));
        
        // Single numbers or names should be ignored
        assert!(!calc.accepts("42"));
        assert!(!calc.accepts("hello"));
    }

    #[test]
    fn test_calculator_query_quadratic() {
        let calc = CalculatorPlugin::new();
        
        // Positive Discriminant
        let res1 = calc.query("mitternachtsformel a 1 b -5 c 6");
        assert_eq!(res1.len(), 1);
        assert_eq!(res1[0].title, "x1 = 3, x2 = 2");

        // Space-free parameters: e.g. a20b30c40
        let res_compact = calc.query("mitternachtsformel a20b30c-40");
        assert_eq!(res_compact.len(), 1);
        assert_eq!(res_compact[0].title, "x1 = 0.85, x2 = -2.35");

        // Space-free parameters and no keyword: e.g. a20b30c-40
        let res_no_kw = calc.query("a20b30c-40");
        assert_eq!(res_no_kw.len(), 1);
        assert_eq!(res_no_kw[0].title, "x1 = 0.85, x2 = -2.35");

        // Zero Discriminant
        let res2 = calc.query("quadratic formula a 1 b -4 c 4");
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].title, "x = 2");

        // Negative Discriminant (Complex Roots)
        let res3 = calc.query("abc-formel a 1 b 2 c 5");
        assert_eq!(res3.len(), 1);
        assert_eq!(res3[0].title, "x1 = -1 + 2i, x2 = -1 - 2i");
    }

    #[test]
    fn test_calculator_query_pq() {
        let calc = CalculatorPlugin::new();

        // Positive Discriminant
        let res1 = calc.query("pq p -5 q 6");
        assert_eq!(res1.len(), 1);
        assert_eq!(res1[0].title, "x1 = 3, x2 = 2");

        // Compact parameters
        let res_compact = calc.query("pq p-5q6");
        assert_eq!(res_compact.len(), 1);
        assert_eq!(res_compact[0].title, "x1 = 3, x2 = 2");

        // Zero Discriminant
        let res2 = calc.query("pq-formel p -4 q 4");
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].title, "x = 2");
    }

    #[test]
    fn test_calculator_query_natural_lang() {
        let calc = CalculatorPlugin::new();
        
        let res = calc.query("5 + 3 * wurzel 16");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "17");

        // Spacing edge cases: multiple spaces between function name and argument
        let res_spaces = calc.query("wurzel    16");
        assert_eq!(res_spaces.len(), 1);
        assert_eq!(res_spaces[0].title, "4");

        // Spacing edge cases: spaces inside parentheses
        let res_paren = calc.query("wurzel ( 25 )");
        assert_eq!(res_paren.len(), 1);
        assert_eq!(res_paren[0].title, "5");

        let res_german = calc.query("10 mal 5 weniger 2");
        assert_eq!(res_german.len(), 1);
        assert_eq!(res_german[0].title, "48");

        // Float division
        let res_div = calc.query("3 / 2");
        assert_eq!(res_div.len(), 1);
        assert_eq!(res_div[0].title, "1.5");
    }
}
