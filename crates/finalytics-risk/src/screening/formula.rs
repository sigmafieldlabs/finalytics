//! Formulas over a screen's measures, fields and earlier formulas: a small language of numbers,
//! names, `+ - * /`, unary minus, parentheses and the functions `abs`, `sqrt`, `log`, `min` and
//! `max`, read into a tree and evaluated. Nothing is executed.

/// A formula read into a tree. Names are resolved to inputs by their place in the slice of
/// values the formula is evaluated on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr {
    Number(f64),
    Input(usize),
    Negate(Box<Expr>),
    Binary(Operator, Box<Expr>, Box<Expr>),
    Call(Function, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Function {
    Abs,
    Sqrt,
    Log,
    Min,
    Max,
}

impl Function {
    fn named(name: &str) -> Option<Function> {
        match name {
            "abs" => Some(Function::Abs),
            "sqrt" => Some(Function::Sqrt),
            "log" => Some(Function::Log),
            "min" => Some(Function::Min),
            "max" => Some(Function::Max),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Name(String),
    Plus,
    Minus,
    Star,
    Slash,
    Open,
    Close,
    Comma,
}

impl Token {
    fn text(&self) -> String {
        match self {
            Token::Number(x) => x.to_string(),
            Token::Name(n) => n.clone(),
            Token::Plus => "+".to_owned(),
            Token::Minus => "-".to_owned(),
            Token::Star => "*".to_owned(),
            Token::Slash => "/".to_owned(),
            Token::Open => "(".to_owned(),
            Token::Close => ")".to_owned(),
            Token::Comma => ",".to_owned(),
        }
    }
}

/// The tokens of `text`, each with the place of its first character, counted from 1.
fn tokens(text: &str) -> Result<Vec<(Token, usize)>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let at = i + 1;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let single = match c {
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            '*' => Some(Token::Star),
            '/' => Some(Token::Slash),
            '(' => Some(Token::Open),
            ')' => Some(Token::Close),
            ',' => Some(Token::Comma),
            _ => None,
        };
        if let Some(token) = single {
            out.push((token, at));
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // An exponent, such as 1e-3.
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let written: String = chars[start..i].iter().collect();
            match written.parse::<f64>() {
                Ok(x) if x.is_finite() => out.push((Token::Number(x), at)),
                _ => return Err(format!("`{written}` at character {at} is not a number")),
            }
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push((Token::Name(chars[start..i].iter().collect()), at));
        } else {
            return Err(format!(
                "`{c}` at character {at} is not part of a formula: use numbers, names, + - * /, \
                 parentheses and abs, sqrt, log, min and max"
            ));
        }
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: Vec<(Token, usize)>,
    next: usize,
    /// The place in the inputs of a name, or why it names none.
    resolve: &'a mut dyn FnMut(&str) -> Result<usize, String>,
    /// Where the formula ends, for a message about something missing at the end.
    end: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.next).map(|(t, _)| t)
    }

    fn at(&self) -> usize {
        self.tokens.get(self.next).map_or(self.end, |(_, at)| *at)
    }

    fn unexpected(&self, wanted: &str) -> String {
        match self.tokens.get(self.next) {
            Some((t, at)) => format!("unexpected `{}` at character {at}: {wanted}", t.text()),
            None => format!("the formula ends at character {}: {wanted}", self.end),
        }
    }

    fn expression(&mut self) -> Result<Expr, String> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => Operator::Add,
                Some(Token::Minus) => Operator::Subtract,
                _ => return Ok(left),
            };
            self.next += 1;
            let right = self.term()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => Operator::Multiply,
                Some(Token::Slash) => Operator::Divide,
                _ => return Ok(left),
            };
            self.next += 1;
            let right = self.unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Minus) {
            self.next += 1;
            return Ok(Expr::Negate(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let at = self.at();
        match self.peek().cloned() {
            Some(Token::Number(x)) => {
                self.next += 1;
                Ok(Expr::Number(x))
            }
            Some(Token::Open) => {
                self.next += 1;
                let inner = self.expression()?;
                if self.peek() != Some(&Token::Close) {
                    return Err(
                        self.unexpected(&format!("close the parenthesis opened at character {at}"))
                    );
                }
                self.next += 1;
                Ok(inner)
            }
            Some(Token::Name(name)) => {
                self.next += 1;
                if self.peek() == Some(&Token::Open) {
                    let Some(function) = Function::named(&name) else {
                        return Err(format!(
                            "unknown function `{name}` at character {at}: the functions are abs, \
                             sqrt, log, min and max"
                        ));
                    };
                    self.next += 1;
                    let mut args = vec![self.expression()?];
                    while self.peek() == Some(&Token::Comma) {
                        self.next += 1;
                        args.push(self.expression()?);
                    }
                    if self.peek() != Some(&Token::Close) {
                        return Err(self.unexpected(&format!(
                            "close the parenthesis of `{name}` at character {at}"
                        )));
                    }
                    self.next += 1;
                    let one = matches!(function, Function::Abs | Function::Sqrt | Function::Log);
                    if one && args.len() != 1 {
                        return Err(format!(
                            "`{name}` at character {at} takes one value, not {}",
                            args.len()
                        ));
                    }
                    if !one && args.len() < 2 {
                        return Err(format!(
                            "`{name}` at character {at} takes two values or more"
                        ));
                    }
                    Ok(Expr::Call(function, args))
                } else {
                    (self.resolve)(&name)
                        .map(Expr::Input)
                        .map_err(|why| format!("`{name}` at character {at}: {why}"))
                }
            }
            _ => Err(self.unexpected("expected a number, a name or a parenthesis")),
        }
    }
}

/// `text` read into a tree, each name resolved to its input by `resolve`; the first problem,
/// with the token and its place, if it cannot be read.
pub(crate) fn parse(
    text: &str,
    resolve: &mut dyn FnMut(&str) -> Result<usize, String>,
) -> Result<Expr, String> {
    let tokens = tokens(text)?;
    if tokens.is_empty() {
        return Err("is empty: give a formula, such as return_1y / volatility".to_owned());
    }
    let mut parser = Parser {
        tokens,
        next: 0,
        resolve,
        end: text.chars().count(),
    };
    let expr = parser.expression()?;
    if parser.next < parser.tokens.len() {
        return Err(parser.unexpected("expected an operator or the end of the formula"));
    }
    Ok(expr)
}

impl Expr {
    /// The value of the formula on `inputs`: absent when an input it uses is, when it divides by
    /// zero, takes the square root of a negative number or the logarithm of one at most zero,
    /// or when the result is not finite.
    pub(crate) fn eval(&self, inputs: &[Option<f64>]) -> Option<f64> {
        let x = match self {
            Expr::Number(x) => *x,
            Expr::Input(k) => inputs.get(*k).copied().flatten()?,
            Expr::Negate(e) => -e.eval(inputs)?,
            Expr::Binary(op, a, b) => {
                let (a, b) = (a.eval(inputs)?, b.eval(inputs)?);
                match op {
                    Operator::Add => a + b,
                    Operator::Subtract => a - b,
                    Operator::Multiply => a * b,
                    Operator::Divide if b == 0.0 => return None,
                    Operator::Divide => a / b,
                }
            }
            Expr::Call(f, args) => {
                let values: Option<Vec<f64>> = args.iter().map(|a| a.eval(inputs)).collect();
                let values = values?;
                match f {
                    Function::Abs => values[0].abs(),
                    Function::Sqrt if values[0] < 0.0 => return None,
                    Function::Sqrt => values[0].sqrt(),
                    Function::Log if values[0] <= 0.0 => return None,
                    Function::Log => values[0].ln(),
                    Function::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
                    Function::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                }
            }
        };
        x.is_finite().then_some(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<Expr, String> {
        let names = ["return_1y", "volatility", "pe"];
        parse(text, &mut |name| {
            names
                .iter()
                .position(|n| *n == name)
                .ok_or_else(|| "unknown name".to_owned())
        })
    }

    fn value(text: &str, inputs: &[Option<f64>]) -> Option<f64> {
        read(text).unwrap().eval(inputs)
    }

    #[test]
    fn precedence_parentheses_and_functions() {
        let x = [Some(0.12), Some(0.2), Some(15.0)];
        assert_eq!(value("1 + 2 * 3", &x), Some(7.0));
        assert_eq!(value("(1 + 2) * 3", &x), Some(9.0));
        assert_eq!(value("-2 * -3", &x), Some(6.0));
        assert_eq!(value("8 / 4 / 2", &x), Some(1.0));
        assert_eq!(value("return_1y / volatility", &x), Some(0.12 / 0.2));
        assert_eq!(value("max(pe, 10, 20) - min(pe, 1)", &x), Some(19.0));
        assert_eq!(value("abs(-3) + sqrt(16) + log(1)", &x), Some(7.0));
        assert_eq!(value("1e-3 * 2.5E2", &x), Some(0.25));
    }

    #[test]
    fn what_has_no_value_gives_none() {
        let x = [None, Some(0.0), Some(-1.0)];
        assert_eq!(value("return_1y + 1", &x), None);
        assert_eq!(value("1 / volatility", &x), None);
        assert_eq!(value("sqrt(pe)", &x), None);
        assert_eq!(value("log(volatility)", &x), None);
    }

    #[test]
    fn problems_name_the_token_and_its_place() {
        let problem = |t: &str| read(t).unwrap_err();
        assert_eq!(
            problem("retrun_1y / 2"),
            "`retrun_1y` at character 1: unknown name"
        );
        assert!(problem("1 +").starts_with("the formula ends at character 3"));
        assert!(problem("(1 + 2").contains("close the parenthesis opened at character 1"));
        assert!(problem("1 2").starts_with("unexpected `2` at character 3"));
        assert!(problem("exp(1)").starts_with("unknown function `exp` at character 1"));
        assert!(problem("sqrt(1, 2)").contains("takes one value, not 2"));
        assert!(problem("max(1)").contains("takes two values or more"));
        assert!(problem("pe > 1").starts_with("`>` at character 4 is not part of a formula"));
        assert!(problem("  ").starts_with("is empty"));
    }
}
