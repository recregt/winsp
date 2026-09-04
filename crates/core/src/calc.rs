use logos::Logos;

const MAX_INPUT_LEN: usize = 1024;

const KNOWN_IDENTIFIERS: &[&str] = &[
    "sqrt", "abs", "sin", "cos", "tan", "ln", "log10", "log", "floor", "ceil", "round", "pi", "e",
];

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]
enum Token {
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
    #[regex(r"[a-zA-Z][a-zA-Z0-9]*", |lex| lex.slice().to_lowercase())]
    Identifier(String),
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

fn segment_identifier(word: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut rest = word;
    while !rest.is_empty() {
        let matched = KNOWN_IDENTIFIERS
            .iter()
            .filter(|name| rest.starts_with(*name))
            .max_by_key(|name| name.len())?;
        parts.push((*matched).to_string());
        rest = &rest[matched.len()..];
    }
    Some(parts)
}

fn expand_identifiers(tokens: Vec<Token>) -> Option<Vec<Token>> {
    let mut out = Vec::with_capacity(tokens.len());
    for tok in tokens {
        match tok {
            Token::Identifier(word) if KNOWN_IDENTIFIERS.contains(&word.as_str()) => {
                out.push(Token::Identifier(word));
            }
            Token::Identifier(word) => {
                for part in segment_identifier(&word)? {
                    out.push(Token::Identifier(part));
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

fn insert_implicit_multiplication(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    for tok in tokens {
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
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Neg,
    Pos,
    Func(String),
    Group,
}

fn precedence(op: &Op) -> u8 {
    match op {
        Op::Add | Op::Sub => 1,
        Op::Mul | Op::Div | Op::Mod => 2,
        Op::Neg | Op::Pos => 3,
        Op::Pow => 4,
        Op::Func(_) | Op::Group => 5,
    }
}

fn is_right_associative(op: &Op) -> bool {
    matches!(op, Op::Pow | Op::Neg | Op::Pos)
}

#[derive(Debug, Clone)]
enum RpnItem {
    Value(f64),
    Op(Op),
}

fn to_rpn(tokens: &[Token]) -> Option<Vec<RpnItem>> {
    let mut output: Vec<RpnItem> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut prev_value_ended = false;

    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            Token::Number(n) => {
                output.push(RpnItem::Value(*n));
                prev_value_ended = true;
            }
            Token::Identifier(name) => {
                if tokens.get(i + 1) == Some(&Token::LParen) && is_function(name) {
                    ops.push(Op::Func(name.clone()));
                    prev_value_ended = false;
                } else {
                    let value = match name.as_str() {
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

fn eval_rpn(rpn: &[RpnItem]) -> Option<f64> {
    let mut stack: Vec<f64> = Vec::new();
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
                let result = match name.as_str() {
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
    let lower = input.to_lowercase();
    let parts: Vec<&str> = lower.split_whitespace().collect();
    if parts.len() == 3 && parts[1] == "of" {
        let pct_str = parts[0].strip_suffix('%')?;
        let pct: f64 = pct_str.parse().ok()?;
        let total: f64 = parts[2].parse().ok()?;
        if !pct.is_finite() || !total.is_finite() {
            return None;
        }
        let result = (pct / 100.0) * total;
        return result.is_finite().then_some(result);
    }
    None
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

    if let Some(res) = try_eval_percentage(trimmed) {
        return Some(format_result(res));
    }

    let tokens: Vec<Token> = Token::lexer(trimmed).collect::<Result<_, _>>().ok()?;
    if tokens.is_empty() {
        return None;
    }

    let tokens = expand_identifiers(tokens)?;
    let tokens = insert_implicit_multiplication(tokens);

    let has_op_or_func = trimmed.contains('E')
        || trimmed.contains('e')
        || tokens.iter().any(|t| {
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
        });
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
        let tokens: Vec<Token> = Token::lexer(&power_chain)
            .collect::<Result<_, _>>()
            .unwrap();
        let tokens = expand_identifiers(tokens).unwrap();
        let tokens = insert_implicit_multiplication(tokens);
        let rpn = to_rpn(&tokens).unwrap();
        assert!(eval_rpn(&rpn).unwrap().is_infinite());

        let deep_parens: String = "(".repeat(500_000) + "1" + &")".repeat(500_000);
        let tokens: Vec<Token> = Token::lexer(&deep_parens)
            .collect::<Result<_, _>>()
            .unwrap();
        let tokens = expand_identifiers(tokens).unwrap();
        let tokens = insert_implicit_multiplication(tokens);
        let rpn = to_rpn(&tokens).unwrap();
        assert_eq!(eval_rpn(&rpn), Some(1.0));
    }
}
