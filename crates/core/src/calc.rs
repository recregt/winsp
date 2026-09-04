use logos::Logos;

const MAX_INPUT_LEN: usize = 1024;

/// Every identifier the calculator knows. Lookups go through
/// [`identifier_bucket`], which groups these by first byte; this list is the
/// reference the buckets are checked against in the tests.
#[cfg(test)]
const KNOWN_IDENTIFIERS: &[&str] = &[
    "sqrt", "abs", "sin", "cos", "tan", "ln", "log10", "log", "floor", "ceil", "round", "pi", "e",
];

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]
enum Token<'a> {
    #[regex(r"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?|\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse().ok())]
    Number(f64),
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[regex(r"[*x×]", priority = 3)]
    Multiply,
    #[regex(r"[/÷]")]
    Divide,
    #[token("%")]
    Modulo,
    #[token("^")]
    Power,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    /// Borrowed from the input, so it is not lowercased yet: every comparison
    /// on it is ASCII case insensitive, and expansion replaces it with the
    /// canonical spelling from `KNOWN_IDENTIFIERS`.
    #[regex(r"[a-zA-Z][a-zA-Z0-9]*", |lex| lex.slice())]
    Identifier(&'a str),
}

fn is_function(name: &str) -> bool {
    matches!(
        name,
        "sqrt"
            | "abs"
            | "sin"
            | "cos"
            | "tan"
            | "ln"
            | "log"
            | "log10"
            | "floor"
            | "ceil"
            | "round"
    )
}

/// Whether `input` can be the start of an expression. An expression starts with
/// a number, a `.`, an opening parenthesis, a unary sign or a known identifier,
/// so anything else is a query for the index and is rejected here rather than
/// after lexing it.
fn starts_like_expression(input: &str) -> bool {
    let Some(&first) = input.as_bytes().first() else {
        return false;
    };
    match first {
        b'0'..=b'9' | b'.' | b'(' | b'+' | b'-' => true,
        // A leading word only evaluates when it starts with a known identifier,
        // which is the same check the segmentation does on its first step, so
        // an unknown word never reaches the lexer.
        _ => known_identifier_prefix(input).is_some(),
    }
}

/// The known identifiers that can start with `first`, longest first so that the
/// longest one wins, as it did when the whole list was scanned. Grouping them
/// keeps a lookup down to at most three comparisons.
fn identifier_bucket(first: u8) -> Option<&'static [&'static str]> {
    let bucket: &'static [&'static str] = match first.to_ascii_lowercase() {
        b's' => &["sqrt", "sin"],
        b'a' => &["abs"],
        b'c' => &["ceil", "cos"],
        b't' => &["tan"],
        b'l' => &["log10", "log", "ln"],
        b'f' => &["floor"],
        b'r' => &["round"],
        b'p' => &["pi"],
        b'e' => &["e"],
        _ => return None,
    };
    Some(bucket)
}

fn known_identifier_prefix(rest: &str) -> Option<&'static str> {
    let bytes = rest.as_bytes();
    identifier_bucket(*bytes.first()?)?
        .iter()
        .copied()
        .find(|name| {
            bytes.len() >= name.len() && bytes[..name.len()].eq_ignore_ascii_case(name.as_bytes())
        })
}

fn push_segmented(out: &mut Vec<Token<'_>>, word: &str) -> Option<()> {
    let mut rest = word;
    while !rest.is_empty() {
        let matched = known_identifier_prefix(rest)?;
        push_token(out, Token::Identifier(matched));
        rest = &rest[matched.len()..];
    }
    Some(())
}

fn push_expanded<'a>(out: &mut Vec<Token<'a>>, tok: Token<'a>) -> Option<()> {
    match tok {
        Token::Identifier(word) => push_segmented(out, word)?,
        other => push_token(out, other),
    }
    Some(())
}

/// Lexes `input`, expands run-together identifiers and inserts the implicit
/// multiplications in a single pass, so a rejected query stops at the first
/// token it cannot handle and a valid one is only collected once.
///
/// Every token spans at least one byte, so the input length bounds how many
/// there can be and the buffer is taken in one allocation instead of growing
/// through four for an ordinary expression.
fn tokenize(input: &str) -> Option<Vec<Token<'_>>> {
    let mut out = Vec::with_capacity(input.len());
    for tok in Token::lexer(input) {
        push_expanded(&mut out, tok.ok()?)?;
    }
    Some(out)
}

/// Pushes `tok`, preceded by the multiplication it implies when it starts a
/// value right after one ended, as in `2pi` or `(1+2)(3)`.
fn push_token<'a>(out: &mut Vec<Token<'a>>, tok: Token<'a>) {
    if let Some(prev) = out.last() {
        let value_ends = matches!(
            prev,
            Token::Number(_) | Token::Identifier(_) | Token::RParen
        );
        let value_starts = match &tok {
            Token::Number(_) => !matches!(prev, Token::Number(_)),
            Token::Identifier(_) => true,
            Token::LParen => !matches!(prev, Token::Identifier(name) if is_function(name)),
            _ => false,
        };
        if value_ends && value_starts {
            out.push(Token::Multiply);
        }
    }
    out.push(tok);
}

#[derive(Debug, Clone, PartialEq)]
enum Op<'a> {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Neg,
    Pos,
    Func(&'a str),
    Group,
}

fn precedence(op: &Op<'_>) -> u8 {
    match op {
        Op::Add | Op::Sub => 1,
        Op::Mul | Op::Div | Op::Mod => 2,
        Op::Neg | Op::Pos => 3,
        Op::Pow => 4,
        Op::Func(_) | Op::Group => 5,
    }
}

fn is_right_associative(op: &Op<'_>) -> bool {
    matches!(op, Op::Pow | Op::Neg | Op::Pos)
}

#[derive(Debug, Clone)]
enum RpnItem<'a> {
    Value(f64),
    Op(Op<'a>),
}

fn to_rpn<'a>(tokens: &[Token<'a>]) -> Option<Vec<RpnItem<'a>>> {
    let mut output: Vec<RpnItem<'a>> = Vec::with_capacity(tokens.len());
    let mut ops: Vec<Op<'a>> = Vec::with_capacity(tokens.len());
    let mut prev_value_ended = false;

    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            Token::Number(n) => {
                output.push(RpnItem::Value(*n));
                prev_value_ended = true;
            }
            Token::Identifier(name) => {
                if tokens.get(i + 1) == Some(&Token::LParen) && is_function(name) {
                    ops.push(Op::Func(name));
                    prev_value_ended = false;
                } else {
                    let value = match *name {
                        "pi" => std::f64::consts::PI,
                        "e" => std::f64::consts::E,
                        _ => return None,
                    };
                    output.push(RpnItem::Value(value));
                    prev_value_ended = true;
                }
            }
            Token::LParen => {
                ops.push(Op::Group);
                prev_value_ended = false;
            }
            Token::RParen => {
                loop {
                    match ops.pop() {
                        Some(Op::Group) => break,
                        Some(op) => output.push(RpnItem::Op(op)),
                        None => return None,
                    }
                }
                if matches!(ops.last(), Some(Op::Func(_))) {
                    output.push(RpnItem::Op(ops.pop()?));
                }
                prev_value_ended = true;
            }
            Token::Plus
            | Token::Minus
            | Token::Multiply
            | Token::Divide
            | Token::Modulo
            | Token::Power => {
                let op = if prev_value_ended {
                    match tok {
                        Token::Plus => Op::Add,
                        Token::Minus => Op::Sub,
                        Token::Multiply => Op::Mul,
                        Token::Divide => Op::Div,
                        Token::Modulo => Op::Mod,
                        Token::Power => Op::Pow,
                        _ => unreachable!(),
                    }
                } else {
                    match tok {
                        Token::Plus => Op::Pos,
                        Token::Minus => Op::Neg,
                        _ => return None,
                    }
                };

                if !matches!(op, Op::Neg | Op::Pos) {
                    while let Some(top) = ops.last() {
                        if matches!(top, Op::Group) {
                            break;
                        }
                        let should_pop = precedence(top) > precedence(&op)
                            || (precedence(top) == precedence(&op) && !is_right_associative(&op));
                        if !should_pop {
                            break;
                        }
                        output.push(RpnItem::Op(ops.pop()?));
                    }
                }
                ops.push(op);
                prev_value_ended = false;
            }
        }
    }

    while let Some(op) = ops.pop() {
        if matches!(op, Op::Group) {
            return None;
        }
        output.push(RpnItem::Op(op));
    }

    Some(output)
}

fn eval_rpn(rpn: &[RpnItem<'_>]) -> Option<f64> {
    let mut stack: Vec<f64> = Vec::with_capacity(rpn.len());
    for item in rpn {
        match item {
            RpnItem::Value(v) => stack.push(*v),
            RpnItem::Op(Op::Neg) => {
                let v = stack.pop()?;
                stack.push(-v);
            }
            RpnItem::Op(Op::Pos) => {}
            RpnItem::Op(Op::Func(name)) => {
                let v = stack.pop()?;
                let result = match *name {
                    "sqrt" if v >= 0.0 => v.sqrt(),
                    "abs" => v.abs(),
                    "sin" => v.sin(),
                    "cos" => v.cos(),
                    "tan" if v.cos().abs() > 1e-9 => v.tan(),
                    "ln" if v > 0.0 => v.ln(),
                    "log" | "log10" if v > 0.0 => v.log10(),
                    "floor" => v.floor(),
                    "ceil" => v.ceil(),
                    "round" => v.round(),
                    _ => return None,
                };
                stack.push(result);
            }
            RpnItem::Op(Op::Group) => return None,
            RpnItem::Op(binary_op) => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                let result = match binary_op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div if b != 0.0 => a / b,
                    Op::Mod if b != 0.0 => a % b,
                    Op::Pow => a.powf(b),
                    _ => return None,
                };
                stack.push(result);
            }
        }
    }
    if stack.len() == 1 { stack.pop() } else { None }
}

fn try_eval_percentage(input: &str) -> Option<f64> {
    let mut parts = input.split_whitespace();
    let pct_str = parts.next()?;
    let of_str = parts.next()?;
    let total_str = parts.next()?;
    if parts.next().is_some() || !of_str.eq_ignore_ascii_case("of") {
        return None;
    }
    let pct_str = pct_str.strip_suffix('%')?;
    let pct: f64 = pct_str.parse().ok()?;
    let total: f64 = total_str.parse().ok()?;
    if !pct.is_finite() || !total.is_finite() {
        return None;
    }
    let result = (pct / 100.0) * total;
    result.is_finite().then_some(result)
}

fn format_result(val: f64) -> String {
    if (val.fract() == 0.0) && (val.abs() < 1e15) {
        format!("{}", val as i64)
    } else if val.abs() < 1e-6 || val.abs() >= 1e15 {
        format!("{val:e}")
    } else {
        let s = format!("{:.6}", val);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

pub fn eval(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_INPUT_LEN {
        return None;
    }

    // Every keystroke reaches this, and most of them are searches for the
    // index, not expressions. Percentages are covered too: they start with the
    // number the percentage is taken of.
    if !starts_like_expression(trimmed) {
        return None;
    }

    if let Some(res) = try_eval_percentage(trimmed) {
        return Some(format_result(res));
    }

    let tokens = tokenize(trimmed)?;
    if tokens.is_empty() {
        return None;
    }

    // A bare number is a query, not a calculation. The scan for an exponent
    // only has to run when no operator or identifier was lexed, which is the
    // one case where scientific notation hides inside a single number token.
    let has_op_or_func = tokens.iter().any(|t| {
        matches!(
            t,
            Token::Plus
                | Token::Minus
                | Token::Multiply
                | Token::Divide
                | Token::Modulo
                | Token::Power
                | Token::Identifier(_)
        )
    }) || trimmed.bytes().any(|b| b.eq_ignore_ascii_case(&b'e'));
    if !has_op_or_func {
        return None;
    }

    let rpn = to_rpn(&tokens)?;
    let val = eval_rpn(&rpn)?;

    if val.is_nan() || val.is_infinite() {
        return None;
    }

    Some(format_result(val))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(eval("2 + 2"), Some("4".to_string()));
        assert_eq!(eval("10 - 3 * 2"), Some("4".to_string()));
        assert_eq!(eval("(10 - 3) * 2"), Some("14".to_string()));
        assert_eq!(eval("10 / 4"), Some("2.5".to_string()));
        assert_eq!(eval("2 ^ 8"), Some("256".to_string()));
        assert_eq!(eval("2*3+4*5"), Some("26".to_string()));
        assert_eq!(eval("2*(3+4)*5"), Some("70".to_string()));
        assert_eq!(eval("10%3"), Some("1".to_string()));
    }

    #[test]
    fn test_math_functions() {
        assert_eq!(eval("sqrt(144)"), Some("12".to_string()));
        assert_eq!(eval("abs(-42)"), Some("42".to_string()));
        assert_eq!(eval("sin(pi/2)"), Some("1".to_string()));
        assert_eq!(eval("log10(100)"), Some("2".to_string()));
        assert_eq!(eval("floor(4.7)"), Some("4".to_string()));
        assert_eq!(eval("ceil(4.2)"), Some("5".to_string()));
        assert_eq!(eval("round(4.5)"), Some("5".to_string()));
    }

    #[test]
    fn test_percentage() {
        assert_eq!(eval("15% of 200"), Some("30".to_string()));
        assert_eq!(eval("25% of 80"), Some("20".to_string()));
    }

    #[test]
    fn test_percentage_rejects_malformed_and_non_finite_input() {
        assert_eq!(eval("15 of 200"), None);
        assert_eq!(eval("15%% of 200"), None);
        assert_eq!(eval("15%%%% of 200"), None);
        assert_eq!(eval("nan% of 2"), None);
        assert_eq!(eval("inf% of 2"), None);
        assert_eq!(eval("15% of nan"), None);
        assert_eq!(eval("15% of inf"), None);
    }

    #[test]
    fn test_invalid_expressions() {
        assert_eq!(eval("hello"), None);
        assert_eq!(eval("123"), None);
        assert_eq!(eval(""), None);
        assert_eq!(eval("2 +"), None);
        assert_eq!(eval("3.5.2"), None);
        assert_eq!(eval("()"), None);
    }

    #[test]
    fn test_queries_that_cannot_start_an_expression_are_rejected() {
        assert_eq!(eval("visual studio code"), None);
        assert_eq!(eval("word"), None);
        assert_eq!(eval("* 2"), None);
        assert_eq!(eval(") + 1"), None);
        // Still reached: these do start like an expression.
        assert_eq!(eval("+2 + 3"), Some("5".to_string()));
        assert_eq!(eval(".5 + 2"), Some("2.5".to_string()));
        assert_eq!(eval("(1 + 2)"), Some("3".to_string()));
        assert_eq!(eval("+15% of 200"), Some("30".to_string()));
    }

    #[test]
    fn test_domain_errors_return_none_not_panic() {
        assert_eq!(eval("10/0"), None);
        assert_eq!(eval("10%0"), None);
        assert_eq!(eval("sqrt(-1)"), None);
        assert_eq!(eval("ln(-1)"), None);
        assert_eq!(eval("log(-1)"), None);
        assert_eq!(eval("tan(pi/2)"), None);
        assert_eq!(eval("tan(3*pi/2)"), None);
        assert_eq!(eval("tan(-pi/2)"), None);
        assert_eq!(eval("tan(pi/4)"), Some("1".to_string()));
    }

    #[test]
    fn test_tan_domain_check_cosine_threshold_boundary() {
        assert!(eval("tan(pi/2 + 1.1e-9)").is_some());
        assert_eq!(eval("tan(pi/2 + 0.9e-9)"), None);
    }

    #[test]
    fn test_unary_minus_binds_looser_than_power() {
        assert_eq!(eval("-2^2"), Some("-4".to_string()));
        assert_eq!(eval("-3^2"), Some("-9".to_string()));
        assert_eq!(eval("(-3)^2"), Some("9".to_string()));
    }

    #[test]
    fn test_power_is_right_associative() {
        assert_eq!(eval("2^3^2"), Some("512".to_string()));
    }

    #[test]
    fn test_chained_unary_operators() {
        assert_eq!(eval("5 - - 2"), Some("7".to_string()));
        assert_eq!(eval("--5"), Some("5".to_string()));
        assert_eq!(eval("2 * -3"), Some("-6".to_string()));
        assert_eq!(eval("2^-2"), Some("0.25".to_string()));
    }

    #[test]
    fn test_implicit_multiplication() {
        assert_eq!(eval("2(3+4)"), Some("14".to_string()));
        assert_eq!(eval("2(3)(4)"), Some("24".to_string()));
        assert_eq!(eval("(2)(3)"), Some("6".to_string()));
        assert_eq!(eval("2pi"), Some(format_result(2.0 * std::f64::consts::PI)));
        assert_eq!(eval("3log(100)"), Some(format_result(3.0 * 2.0)));
        assert_eq!(
            eval("pilog(100)"),
            Some(format_result(std::f64::consts::PI * 2.0))
        );
        assert_eq!(eval("2log10(100)"), Some(format_result(2.0 * 2.0)));
        assert_eq!(
            eval("sin(pi)cos(pi)"),
            Some(format_result(
                std::f64::consts::PI.sin() * std::f64::consts::PI.cos()
            ))
        );
        assert_eq!(eval("3.5.2"), None);
    }

    #[test]
    fn test_identifiers_are_case_insensitive() {
        assert_eq!(eval("SQRT(144)"), Some("12".to_string()));
        assert_eq!(eval("Log10(100)"), Some("2".to_string()));
        assert_eq!(eval("2PI"), Some(format_result(2.0 * std::f64::consts::PI)));
        assert_eq!(
            eval("PIE"),
            Some(format_result(std::f64::consts::PI * std::f64::consts::E))
        );
    }

    #[test]
    fn test_bare_word_segments_into_known_identifiers_without_parens_or_digits() {
        assert_eq!(
            eval("pie"),
            Some(format_result(std::f64::consts::PI * std::f64::consts::E))
        );
    }

    #[test]
    fn test_known_identifier_prefix_buckets_stay_in_sync_with_known_identifiers() {
        for name in KNOWN_IDENTIFIERS {
            assert_eq!(known_identifier_prefix(name), Some(*name), "name: {name:?}");
            let upper = name.to_uppercase();
            assert_eq!(
                known_identifier_prefix(&upper),
                Some(*name),
                "name: {upper:?}"
            );
            assert!(
                starts_like_expression(name),
                "identifier rejected before lexing: {name:?}"
            );
        }
    }

    /// A bucket holding a name that is no longer known, or holding a short name
    /// before a longer one it prefixes, would silently change what the longest
    /// match is, so both are pinned here.
    #[test]
    fn test_identifier_buckets_hold_only_known_names_longest_first() {
        for first in b'a'..=b'z' {
            let Some(bucket) = identifier_bucket(first) else {
                continue;
            };
            for (i, name) in bucket.iter().enumerate() {
                assert!(
                    KNOWN_IDENTIFIERS.contains(name),
                    "unknown name in bucket {:?}: {name:?}",
                    first as char
                );
                assert_eq!(name.as_bytes()[0], first, "name in wrong bucket: {name:?}");
                for shorter in &bucket[..i] {
                    assert!(
                        shorter.len() >= name.len(),
                        "bucket {:?} is not longest first: {shorter:?} before {name:?}",
                        first as char
                    );
                }
            }
        }
    }

    #[test]
    fn test_scientific_notation_vs_eulers_number() {
        assert_eq!(eval("1.2E2"), Some("120".to_string()));
        assert_eq!(eval("2E-2"), Some("0.02".to_string()));
        assert_eq!(eval("-1.0E-2"), Some("-0.01".to_string()));
        assert_eq!(eval("1e3"), Some("1000".to_string()));
        assert_eq!(eval("2e-2"), Some("0.02".to_string()));
        assert_eq!(eval("-1.0e-2"), Some("-0.01".to_string()));
        assert_eq!(eval("2e"), Some(format_result(2.0 * std::f64::consts::E)));
    }

    #[test]
    fn test_floating_point_display_hides_ieee754_noise() {
        assert_eq!(eval("0.1+0.2"), Some("0.3".to_string()));
        assert_eq!(eval(".5+.5"), Some("1".to_string()));
        assert_eq!(eval("1/3*3"), Some("1".to_string()));
    }

    #[test]
    fn test_very_small_results_use_scientific_notation_instead_of_zero() {
        assert_eq!(format_result(1e-10), "1e-10");
        assert_eq!(format_result(-1e-10), "-1e-10");
        assert_ne!(format_result(1e-10), "0");
        assert_ne!(format_result(-1e-10), "-0");
    }

    #[test]
    fn test_query_longer_than_cap_is_rejected() {
        let too_long = "1".repeat(MAX_INPUT_LEN + 1);
        assert_eq!(eval(&too_long), None);
    }

    #[test]
    fn test_deeply_nested_input_does_not_recurse_even_uncapped() {
        let power_chain: String = "2".to_string() + &"^2".repeat(500_000);
        let tokens = tokenize(&power_chain).unwrap();
        let rpn = to_rpn(&tokens).unwrap();
        assert!(eval_rpn(&rpn).unwrap().is_infinite());

        let deep_parens: String = "(".repeat(500_000) + "1" + &")".repeat(500_000);
        let tokens = tokenize(&deep_parens).unwrap();
        let rpn = to_rpn(&tokens).unwrap();
        assert_eq!(eval_rpn(&rpn), Some(1.0));
    }
}
