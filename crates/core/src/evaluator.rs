#[derive(Debug, PartialEq, Clone)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,
    LParen,
    RParen,
    Identifier(String),
}

pub struct Evaluator;

impl Evaluator {
    pub fn try_eval(input: &str) -> Option<String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(res) = Self::try_eval_percentage(trimmed) {
            return Some(Self::format_result(res));
        }

        let tokens = Self::tokenize(trimmed)?;
        if tokens.is_empty() {
            return None;
        }

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
        });

        if !has_op_or_func {
            return None;
        }

        let mut parser = Parser::new(tokens);
        let val = parser.parse_expression()?;

        if parser.has_more() {
            return None;
        }

        if val.is_nan() || val.is_infinite() {
            return None;
        }

        Some(Self::format_result(val))
    }

    fn try_eval_percentage(input: &str) -> Option<f64> {
        let lower = input.to_lowercase();
        let parts: Vec<&str> = lower.split_whitespace().collect();
        if parts.len() == 3 && parts[1] == "of" {
            let pct_str = parts[0].trim_end_matches('%');
            let pct: f64 = pct_str.parse().ok()?;
            let total: f64 = parts[2].parse().ok()?;
            return Some((pct / 100.0) * total);
        }
        None
    }

    fn format_result(val: f64) -> String {
        if (val.fract() == 0.0) && (val.abs() < 1e15) {
            format!("{}", val as i64)
        } else {
            let s = format!("{:.6}", val);
            let trimmed = s.trim_end_matches('0').trim_end_matches('.');
            trimmed.to_string()
        }
    }

    fn tokenize(input: &str) -> Option<Vec<Token>> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            match c {
                '+' => {
                    tokens.push(Token::Plus);
                    i += 1;
                }
                '-' => {
                    tokens.push(Token::Minus);
                    i += 1;
                }
                '*' | 'x' | '×' => {
                    tokens.push(Token::Multiply);
                    i += 1;
                }
                '/' | '÷' => {
                    tokens.push(Token::Divide);
                    i += 1;
                }
                '%' => {
                    tokens.push(Token::Modulo);
                    i += 1;
                }
                '^' => {
                    tokens.push(Token::Power);
                    i += 1;
                }
                '(' => {
                    tokens.push(Token::LParen);
                    i += 1;
                }
                ')' => {
                    tokens.push(Token::RParen);
                    i += 1;
                }
                '0'..='9' | '.' => {
                    let start = i;
                    let mut has_dot = c == '.';
                    i += 1;
                    while i < chars.len() {
                        if chars[i] == '.' {
                            if has_dot {
                                break;
                            }
                            has_dot = true;
                            i += 1;
                        } else if chars[i].is_ascii_digit() {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    let num = num_str.parse::<f64>().ok()?;
                    tokens.push(Token::Number(num));
                }
                'a'..='z' | 'A'..='Z' => {
                    let start = i;
                    while i < chars.len() && chars[i].is_alphabetic() {
                        i += 1;
                    }
                    let id: String = chars[start..i].iter().collect();
                    tokens.push(Token::Identifier(id.to_lowercase()));
                }
                _ => return None,
            }
        }

        Some(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn has_more(&self) -> bool {
        self.pos < self.tokens.len()
    }

    fn parse_expression(&mut self) -> Option<f64> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Option<f64> {
        let mut left = self.parse_mul_div()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_mul_div()?;
                    left += right;
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_mul_div()?;
                    left -= right;
                }
                _ => break,
            }
        }
        Some(left)
    }

    fn parse_mul_div(&mut self) -> Option<f64> {
        let mut left = self.parse_power()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Multiply => {
                    self.advance();
                    let right = self.parse_power()?;
                    left *= right;
                }
                Token::Divide => {
                    self.advance();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None;
                    }
                    left /= right;
                }
                Token::Modulo => {
                    self.advance();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None;
                    }
                    left %= right;
                }
                _ => break,
            }
        }
        Some(left)
    }

    fn parse_power(&mut self) -> Option<f64> {
        let left = self.parse_unary()?;

        if let Some(Token::Power) = self.peek() {
            self.advance();
            let right = self.parse_power()?;
            return Some(left.powf(right));
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<f64> {
        if let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.advance();
                    self.parse_unary()
                }
                Token::Minus => {
                    self.advance();
                    let val = self.parse_unary()?;
                    Some(-val)
                }
                _ => self.parse_primary(),
            }
        } else {
            None
        }
    }

    fn parse_primary(&mut self) -> Option<f64> {
        let tok = self.advance()?;
        match tok {
            Token::Number(n) => Some(n),
            Token::Identifier(id) => match id.as_str() {
                "pi" => Some(std::f64::consts::PI),
                "e" => Some(std::f64::consts::E),
                func => {
                    if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let arg = self.parse_expression()?;
                        if self.advance() != Some(Token::RParen) {
                            return None;
                        }
                        match func {
                            "sqrt" => {
                                if arg >= 0.0 {
                                    Some(arg.sqrt())
                                } else {
                                    None
                                }
                            }
                            "abs" => Some(arg.abs()),
                            "sin" => Some(arg.sin()),
                            "cos" => Some(arg.cos()),
                            "tan" => Some(arg.tan()),
                            "ln" => {
                                if arg > 0.0 {
                                    Some(arg.ln())
                                } else {
                                    None
                                }
                            }
                            "log" | "log10" => {
                                if arg > 0.0 {
                                    Some(arg.log10())
                                } else {
                                    None
                                }
                            }
                            "floor" => Some(arg.floor()),
                            "ceil" => Some(arg.ceil()),
                            "round" => Some(arg.round()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
            },
            Token::LParen => {
                let val = self.parse_expression()?;
                if self.advance() != Some(Token::RParen) {
                    return None;
                }
                Some(val)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(Evaluator::try_eval("2 + 2"), Some("4".to_string()));
        assert_eq!(Evaluator::try_eval("10 - 3 * 2"), Some("4".to_string()));
        assert_eq!(Evaluator::try_eval("(10 - 3) * 2"), Some("14".to_string()));
        assert_eq!(Evaluator::try_eval("10 / 4"), Some("2.5".to_string()));
        assert_eq!(Evaluator::try_eval("2 ^ 8"), Some("256".to_string()));
    }

    #[test]
    fn test_math_functions() {
        assert_eq!(Evaluator::try_eval("sqrt(144)"), Some("12".to_string()));
        assert_eq!(Evaluator::try_eval("abs(-42)"), Some("42".to_string()));
    }

    #[test]
    fn test_percentage() {
        assert_eq!(Evaluator::try_eval("15% of 200"), Some("30".to_string()));
        assert_eq!(Evaluator::try_eval("25% of 80"), Some("20".to_string()));
    }

    #[test]
    fn test_invalid_expressions() {
        assert_eq!(Evaluator::try_eval("hello"), None);
        assert_eq!(Evaluator::try_eval("123"), None);
        assert_eq!(Evaluator::try_eval(""), None);
        assert_eq!(Evaluator::try_eval("2 +"), None);
    }
}
