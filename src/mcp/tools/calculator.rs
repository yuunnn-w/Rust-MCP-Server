use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalculatorParams {
    /// Mathematical expression to evaluate (supports +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e)
    #[schemars(description = "Mathematical expression to evaluate")]
    pub expression: String,
}

pub async fn calculator(params: Parameters<CalculatorParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let result = evaluate_expression(&params.expression)?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("Result: {}", result),
    )]))
}

fn evaluate_expression(expr: &str) -> Result<f64, String> {
    // Remove whitespace
    let expr: String = expr.chars().filter(|c| !c.is_whitespace()).collect();

    if expr.is_empty() {
        return Err("Empty expression".to_string());
    }

    // Tokenize and evaluate
    let tokens = tokenize(&expr)?;
    let result = parse_and_evaluate(&tokens)?;

    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    LeftParen,
    RightParen,
    Identifier(String),
    Comma,
}

fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '0'..='9' | '.' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let num: f64 = num_str
                    .parse()
                    .map_err(|_| format!("Invalid number: {}", num_str))?;
                tokens.push(Token::Number(num));
            }
            '+' => {
                tokens.push(Token::Plus);
                chars.next();
            }
            '-' => {
                tokens.push(Token::Minus);
                chars.next();
            }
            '*' => {
                tokens.push(Token::Multiply);
                chars.next();
            }
            '/' => {
                tokens.push(Token::Divide);
                chars.next();
            }
            '^' => {
                tokens.push(Token::Power);
                chars.next();
            }
            '(' => {
                tokens.push(Token::LeftParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RightParen);
                chars.next();
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() || c == '_' || c.is_ascii_digit() {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Identifier(ident));
            }
            _ => return Err(format!("Invalid character: {}", ch)),
        }
    }

    Ok(tokens)
}

fn parse_and_evaluate(tokens: &[Token]) -> Result<f64, String> {
    let (result, _) = parse_expression(tokens, 0)?;
    Ok(result)
}

fn parse_expression(tokens: &[Token], pos: usize) -> Result<(f64, usize), String> {
    parse_add_sub(tokens, pos)
}

fn parse_add_sub(tokens: &[Token], pos: usize) -> Result<(f64, usize), String> {
    let (mut left, mut pos) = parse_mul_div(tokens, pos)?;

    while pos < tokens.len() {
        match &tokens[pos] {
            Token::Plus => {
                let (right, new_pos) = parse_mul_div(tokens, pos + 1)?;
                left += right;
                pos = new_pos;
            }
            Token::Minus => {
                let (right, new_pos) = parse_mul_div(tokens, pos + 1)?;
                left -= right;
                pos = new_pos;
            }
            _ => break,
        }
    }

    Ok((left, pos))
}

fn parse_mul_div(tokens: &[Token], pos: usize) -> Result<(f64, usize), String> {
    let (mut left, mut pos) = parse_power(tokens, pos)?;

    while pos < tokens.len() {
        match &tokens[pos] {
            Token::Multiply => {
                let (right, new_pos) = parse_power(tokens, pos + 1)?;
                left *= right;
                pos = new_pos;
            }
            Token::Divide => {
                let (right, new_pos) = parse_power(tokens, pos + 1)?;
                if right == 0.0 {
                    return Err("Division by zero".to_string());
                }
                left /= right;
                pos = new_pos;
            }
            _ => break,
        }
    }

    Ok((left, pos))
}

fn parse_power(tokens: &[Token], pos: usize) -> Result<(f64, usize), String> {
    let (left, pos) = parse_unary(tokens, pos)?;

    if pos < tokens.len() && tokens[pos] == Token::Power {
        let (right, new_pos) = parse_power(tokens, pos + 1)?;
        Ok((left.powf(right), new_pos))
    } else {
        Ok((left, pos))
    }
}

fn parse_unary(tokens: &[Token], pos: usize) -> Result<(f64, usize), String> {
    if pos >= tokens.len() {
        return Err("Unexpected end of expression".to_string());
    }

    match &tokens[pos] {
        Token::Plus => parse_primary(tokens, pos + 1),
        Token::Minus => {
            let (val, new_pos) = parse_primary(tokens, pos + 1)?;
            Ok((-val, new_pos))
        }
        _ => parse_primary(tokens, pos),
    }
}

fn parse_primary(tokens: &[Token], pos: usize) -> Result<(f64, usize), String> {
    if pos >= tokens.len() {
        return Err("Unexpected end of expression".to_string());
    }

    match &tokens[pos] {
        Token::Number(n) => Ok((*n, pos + 1)),
        Token::Identifier(name) => {
            // Check if it's a function call or constant
            if pos + 1 < tokens.len() && tokens[pos + 1] == Token::LeftParen {
                // Function call
                parse_function(tokens, pos, name)
            } else {
                // Constant
                parse_constant(name).map(|v| (v, pos + 1))
            }
        }
        Token::LeftParen => {
            let (val, new_pos) = parse_expression(tokens, pos + 1)?;
            if new_pos >= tokens.len() || tokens[new_pos] != Token::RightParen {
                return Err("Missing closing parenthesis".to_string());
            }
            Ok((val, new_pos + 1))
        }
        _ => Err(format!("Unexpected token: {:?}", tokens[pos])),
    }
}

fn parse_function(tokens: &[Token], pos: usize, name: &str) -> Result<(f64, usize), String> {
    // pos is at identifier, pos+1 should be '('
    let mut pos = pos + 2; // Skip identifier and '('
    let mut args = Vec::new();

    // Parse arguments
    if pos < tokens.len() && tokens[pos] != Token::RightParen {
        loop {
            let (arg, new_pos) = parse_expression(tokens, pos)?;
            args.push(arg);
            pos = new_pos;

            if pos >= tokens.len() {
                return Err("Unexpected end of expression".to_string());
            }

            match &tokens[pos] {
                Token::Comma => {
                    pos += 1;
                }
                Token::RightParen => break,
                _ => return Err("Expected ',' or ')'".to_string()),
            }
        }
    }

    if pos >= tokens.len() || tokens[pos] != Token::RightParen {
        return Err("Missing closing parenthesis".to_string());
    }
    pos += 1; // Skip ')'

    // Evaluate function
    let result = match name.to_lowercase().as_str() {
        "sqrt" => {
            if args.len() != 1 {
                return Err("sqrt takes 1 argument".to_string());
            }
            if args[0] < 0.0 {
                return Err("Cannot compute square root of negative number".to_string());
            }
            args[0].sqrt()
        }
        "sin" => {
            if args.len() != 1 {
                return Err("sin takes 1 argument".to_string());
            }
            args[0].to_radians().sin()
        }
        "cos" => {
            if args.len() != 1 {
                return Err("cos takes 1 argument".to_string());
            }
            args[0].to_radians().cos()
        }
        "tan" => {
            if args.len() != 1 {
                return Err("tan takes 1 argument".to_string());
            }
            args[0].to_radians().tan()
        }
        "log" => {
            if args.len() != 1 {
                return Err("log takes 1 argument".to_string());
            }
            if args[0] <= 0.0 {
                return Err("Cannot compute logarithm of non-positive number".to_string());
            }
            args[0].log10()
        }
        "ln" => {
            if args.len() != 1 {
                return Err("ln takes 1 argument".to_string());
            }
            if args[0] <= 0.0 {
                return Err("Cannot compute natural logarithm of non-positive number".to_string());
            }
            args[0].ln()
        }
        "abs" => {
            if args.len() != 1 {
                return Err("abs takes 1 argument".to_string());
            }
            args[0].abs()
        }
        "min" => {
            if args.is_empty() {
                return Err("min takes at least 1 argument".to_string());
            }
            args.into_iter().fold(f64::INFINITY, f64::min)
        }
        "max" => {
            if args.is_empty() {
                return Err("max takes at least 1 argument".to_string());
            }
            args.into_iter().fold(f64::NEG_INFINITY, f64::max)
        }
        "floor" => {
            if args.len() != 1 {
                return Err("floor takes 1 argument".to_string());
            }
            args[0].floor()
        }
        "ceil" => {
            if args.len() != 1 {
                return Err("ceil takes 1 argument".to_string());
            }
            args[0].ceil()
        }
        "round" => {
            if args.len() != 1 {
                return Err("round takes 1 argument".to_string());
            }
            args[0].round()
        }
        _ => return Err(format!("Unknown function: {}", name)),
    };

    Ok((result, pos))
}

fn parse_constant(name: &str) -> Result<f64, String> {
    match name.to_lowercase().as_str() {
        "pi" => Ok(std::f64::consts::PI),
        "e" => Ok(std::f64::consts::E),
        _ => Err(format!("Unknown constant: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_operations() {
        let test_cases = vec![
            ("2 + 3", 5.0),
            ("10 - 4", 6.0),
            ("3 * 4", 12.0),
            ("15 / 3", 5.0),
            ("2 ^ 3", 8.0),
        ];

        for (expr, expected) in test_cases {
            let params = CalculatorParams {
                expression: expr.to_string(),
            };
            let result = calculator(Parameters(params)).await.unwrap();
            if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
                let val: f64 = text.text.split_whitespace().last().unwrap().parse().unwrap();
                assert!((val - expected).abs() < 1e-10, "{}: got {}, expected {}", expr, val, expected);
            }
        }
    }

    #[tokio::test]
    async fn test_functions() {
        let params = CalculatorParams {
            expression: "sqrt(16)".to_string(),
        };
        let result = calculator(Parameters(params)).await.unwrap();
        if let Some(text) = result.content.first().and_then(|c| c.as_text()) {
            assert!(text.text.contains("4"));
        }
    }

    #[tokio::test]
    async fn test_constants() {
        let params = CalculatorParams {
            expression: "pi * 2".to_string(),
        };
        let result = calculator(Parameters(params)).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_tokenize() {
        // Tokenize doesn't handle whitespace - it should be filtered before calling
        let tokens = tokenize("2+3*4").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], Token::Number(2.0));
        assert_eq!(tokens[1], Token::Plus);
        assert_eq!(tokens[2], Token::Number(3.0));
        assert_eq!(tokens[3], Token::Multiply);
        assert_eq!(tokens[4], Token::Number(4.0));
    }
}
