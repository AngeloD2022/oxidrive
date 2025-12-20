//!
//! HyperDrive Install Condition DSL Evaluator
//!

use crate::core::condition::HDConditionError::{EvaluationError, LexError, ParseError};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HDConditionError {
    #[error("Error lexing input: {0}")]
    LexError(String),
    #[error("Error parsing input: {0}")]
    ParseError(String),
    #[error("Error evaluating input: {0}")]
    EvaluationError(String),
}

pub type ConditionResult<T> = Result<T, HDConditionError>;

#[derive(Eq, PartialEq)]
struct Identifier(String);

#[derive(Eq, PartialEq)]
struct Value(String);

#[derive(Copy, Clone)]
pub enum Operation {
    Lt,
    Gt,
    Geq,
    Leq,
    Eq,
    Or,
    And,
}

impl Operation {
    pub fn is_cmp(&self) -> bool {
        match self {
            Operation::Lt | Operation::Gt | Operation::Geq | Operation::Leq | Operation::Eq => true,
            Operation::Or | Operation::And => false,
        }
    }

    pub fn is_ineq(&self) -> bool {
        match self {
            Operation::Geq | Operation::Gt | Operation::Lt | Operation::Leq => true,
            _ => false,
        }
    }
}

enum ConditionToken {
    Ident(String),
    Val(String),
    Op(Operation),
    Eof,
}

struct ConditionLexer {
    pos: usize,
    buf: Vec<char>,
}

impl ConditionLexer {
    pub fn new(input: &str) -> Self {
        let chars = input.chars().collect();

        Self { pos: 0, buf: chars }
    }

    fn peek(&mut self, n: u64) -> Option<char> {
        if self.pos + n as usize >= self.buf.len() {
            None
        } else {
            Some(self.buf[self.pos + n as usize])
        }
    }

    fn advance(&mut self, n: u64) {
        self.pos += n as usize;
    }

    fn read_identifier(&mut self) -> ConditionResult<ConditionToken> {
        // precond: current pos is on '['

        if let Some(c) = self.peek(0) {
            if c != '[' {
                return Err(LexError(format!("Unexpected token {} @ {}", c, self.pos)));
            }
            self.advance(1);
        } else {
            return Err(LexError("Unexpected EOF".to_owned()));
        }

        let start = self.pos;
        while let Some(c) = self.peek(0)
            && c != ']'
        {
            self.advance(1);
        }
        let s: String = self.buf[start..self.pos].iter().collect();
        self.advance(1);

        Ok(ConditionToken::Ident(s))
    }

    fn read_operator(&mut self) -> ConditionResult<ConditionToken> {
        let first = self.peek(0).ok_or(LexError("Unexpected EOF".to_string()))?;

        let m = (first, self.peek(1));

        let operation = match m {
            ('<', Some('=')) => {
                self.advance(2);
                Operation::Leq
            }
            ('<', _) => {
                self.advance(1);
                Operation::Lt
            }
            ('>', Some('=')) => {
                self.advance(2);
                Operation::Geq
            }
            ('>', _) => {
                self.advance(1);
                Operation::Gt
            }
            ('=', Some('=')) => {
                self.advance(2);
                Operation::Eq
            }
            ('|', Some('|')) => {
                self.advance(2);
                Operation::Or
            }
            ('&', Some('&')) => {
                self.advance(2);
                Operation::And
            }
            _ => return Err(LexError(format!("Invalid operation @ {}", self.pos))),
        };

        Ok(ConditionToken::Op(operation))
    }

    fn read_value(&mut self) -> ConditionResult<ConditionToken> {
        const ILLEGAL: &str = "><=|&[";

        if let Some(c) = self.peek(0) {
            if ILLEGAL.contains(c) {
                return Err(LexError(format!("Illegal operator @ {}", self.pos)));
            }
        } else {
            return Err(LexError("Unexpected EOF".to_owned()));
        }

        let start = self.pos;
        while let Some(c) = self.peek(0)
            && !ILLEGAL.contains(c)
        {
            self.advance(1);
        }

        let value: String = self.buf[start..self.pos].iter().collect();
        Ok(ConditionToken::Val(value))
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek(0)
            && c.is_whitespace()
        {
            self.advance(1);
        }
    }

    fn next_token(&mut self) -> ConditionResult<ConditionToken> {
        const OPERATORS: &str = "><=|&";

        if let Some(c) = self.peek(0)
            && c.is_whitespace()
        {
            self.skip_whitespace();
        }

        if let Some(c) = self.peek(0) {
            if OPERATORS.contains(c) {
                return self.read_operator();
            } else if c == '[' {
                return self.read_identifier();
            } else {
                return self.read_value();
            }
        }

        Err(LexError("Unexpected EOF".to_owned()))
    }

    pub fn tokenize(&mut self) -> ConditionResult<Vec<ConditionToken>> {
        let mut tokens = Vec::new();

        while self.pos < self.buf.len() {
            tokens.push(self.next_token()?)
        }

        Ok(tokens)
    }
}

pub enum ExpressionNode {
    And(Box<ExpressionNode>, Box<ExpressionNode>),
    Or(Box<ExpressionNode>, Box<ExpressionNode>),
    Equality(Identifier, Value),
    Inequality(Identifier, Operation, Value),
}

struct ConditionParser {
    tokens: Vec<ConditionToken>,
    pos: usize,
}

impl ConditionParser {
    fn new(tokens: Vec<ConditionToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn current_token(&self) -> &ConditionToken {
        self.tokens.get(self.pos).unwrap_or(&ConditionToken::Eof)
    }

    fn parse_or(&mut self) -> ConditionResult<ExpressionNode> {
        let mut left = self.parse_and()?;

        while let ConditionToken::Op(Operation::Or) = self.current_token() {
            self.advance();
            let right = self.parse_and()?;
            left = ExpressionNode::Or(Box::new(left), Box::new(right))
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> ConditionResult<ExpressionNode> {
        let mut left = self.parse_comparison()?;

        while let ConditionToken::Op(Operation::And) = self.current_token() {
            self.advance();
            let right = self.parse_comparison()?;
            left = ExpressionNode::And(Box::new(left), Box::new(right))
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> ConditionResult<ExpressionNode> {
        let token = self.current_token().clone();

        let ident = match token {
            ConditionToken::Ident(s) => s.clone(),
            _ => return Err(ParseError("Expected identifier".to_string())),
        };
        self.advance();

        let op_token = self.current_token().clone();
        let operation = match op_token {
            ConditionToken::Op(op) => op.clone(),
            _ => return Err(ParseError("Expected operator".to_string())),
        };
        self.advance();

        let val_token = self.current_token().clone();
        let v = match val_token {
            ConditionToken::Val(s) => s.clone(),
            _ => return Err(ParseError("Expected value".to_string())),
        };
        self.advance();

        if !operation.is_cmp() {
            return Err(ParseError("Invalid operator".to_string()));
        }

        let expr = if operation.is_ineq() {
            ExpressionNode::Inequality(Identifier(ident), operation, Value(v.trim().into()))
        } else {
            ExpressionNode::Equality(Identifier(ident), Value(v.trim().into()))
        };

        Ok(expr)
    }

    pub fn parse(&mut self) -> ConditionResult<ExpressionNode> {
        self.parse_or()
    }
}

pub fn compare_version(a: &str, b: &str, op: &Operation) -> bool {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    let max_len = a_parts.len().max(b_parts.len());

    for i in 0..max_len {
        let a_val = a_parts.get(i).copied().unwrap_or(0);
        let b_val = b_parts.get(i).copied().unwrap_or(0);

        if a_val != b_val {
            return match op {
                Operation::Lt => a_val < b_val,
                Operation::Gt => a_val > b_val,
                Operation::Geq => a_val >= b_val,
                Operation::Leq => a_val <= b_val,
                Operation::Eq => a_val == b_val,
                _ => false,
            };
        }
    }

    matches!(op, Operation::Eq | Operation::Leq | Operation::Geq)
}

pub enum EvalValue {
    Version(String),
    String(String),
    StringSet(Vec<String>),
    // todo: add a string list type and make the result of equals be if it contains the right operand.
    //  Technically, the installLanguage var should be this type, since ppl might wanna install
    //  multiple langs.
    Bool(bool),
    Number(i32),
}

impl EvalValue {
    fn compare(&self, op: &Operation, b: &Self) -> ConditionResult<bool> {
        match (self, b) {
            (EvalValue::Version(a), EvalValue::Version(b)) => Ok(compare_version(a, b, op)),
            (EvalValue::Number(a), EvalValue::Number(b)) => Ok(match op {
                Operation::Lt => a < b,
                Operation::Gt => a > b,
                Operation::Geq => a >= b,
                Operation::Leq => a <= b,
                Operation::Eq => a == b,
                _ => false,
            }),
            (EvalValue::String(a), EvalValue::String(b)) => match op {
                Operation::Eq => Ok(a == b),
                _ => Err(EvaluationError("Strings only support equality".to_string())),
            },
            (EvalValue::StringSet(a), EvalValue::String(b)) => match op {
                Operation::Eq => Ok(a.contains(b)),
                _ => Err(EvaluationError(
                    "String lists only support equality.".to_string(),
                )),
            },
            (EvalValue::Bool(a), EvalValue::Bool(b)) => match op {
                Operation::Eq => Ok(a == b),
                _ => Err(EvaluationError("Bools only support equality".to_string())),
            },
            _ => Err(EvaluationError("Type mismatch in comparison".to_string())),
        }
    }

    fn coerce_str(&self, value: &str) -> Self {
        match self {
            EvalValue::Version(_) => EvalValue::Version(value.to_string()),
            EvalValue::String(_) | EvalValue::StringSet(_) => EvalValue::String(value.to_string()),
            EvalValue::Bool(_) => EvalValue::Bool(value == "true"),
            EvalValue::Number(_) => EvalValue::Number(i32::from_str(value).unwrap()),
        }
    }
}

pub struct ConditionEvaluator {
    variables: HashMap<String, EvalValue>,
    strict: bool,
}

impl ConditionEvaluator {
    pub fn new(vars: HashMap<String, EvalValue>, strict_mode: bool) -> Self {
        Self {
            variables: vars,
            strict: strict_mode,
        }
    }

    fn get_var(&self, ident: &str) -> Option<&EvalValue> {
        self.variables.get(&ident.to_string())
    }

    pub fn evaluate(&self, expression: &ExpressionNode) -> ConditionResult<bool> {
        match expression {
            ExpressionNode::And(left, right) => Ok(self.evaluate(left)? && self.evaluate(right)?),
            ExpressionNode::Or(left, right) => Ok(self.evaluate(left)? || self.evaluate(right)?),
            ExpressionNode::Equality(ident, value) => {
                if let Some(var) = self.get_var(&ident.0) {
                    let b = var.coerce_str(&value.0);
                    Ok(var.compare(&Operation::Eq, &b)?)
                } else if !self.strict {
                    Ok(true)
                } else {
                    Err(EvaluationError(format!(
                        "Undeclared identifier: {}",
                        ident.0
                    )))
                }
            }
            ExpressionNode::Inequality(ident, op, val) => {
                if let Some(var) = self.get_var(&ident.0) {
                    let b = var.coerce_str(&val.0);
                    var.compare(op, &b)
                } else if !self.strict {
                    Ok(true)
                } else {
                    Err(EvaluationError(format!(
                        "Undeclared identifier: {}",
                        ident.0
                    )))
                }
            }
        }
    }
}

pub fn parse_condition(expr: &str) -> ConditionResult<ExpressionNode> {
    let mut lexer = ConditionLexer::new(expr);
    let tokens = lexer.tokenize()?;

    let mut parser = ConditionParser::new(tokens);

    parser.parse()
}

mod tests {
    use crate::core::condition::{ConditionEvaluator, ConditionLexer, ConditionParser, EvalValue};
    use std::collections::HashMap;

    #[test]
    fn test_lexer() {
        let input = "[installLanguage]==cs_CZ||[installLanguage]==da_DK||[installLanguage]==de_DE||[installLanguage]==en_GB||[installLanguage]==en_US||[installLanguage]==es_ES||[installLanguage]==es_MX||[installLanguage]==fi_FI||[installLanguage]==fr_CA||[installLanguage]==fr_FR||[installLanguage]==hu_HU||[installLanguage]==it_IT||[installLanguage]==nb_NO||[installLanguage]==nl_NL||[installLanguage]==pl_PL||[installLanguage]==pt_BR||[installLanguage]==ru_RU||[installLanguage]==sv_SE||[installLanguage]==tr_TR||[installLanguage]==uk_UA";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();

        let input = "[OSProcessorFamily]==64-bit&&[OSVersion]<10.14 &&[OSVersion]>=10.13";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
    }

    #[test]
    fn test_parser() {
        let input = "[installLanguage]==cs_CZ||[installLanguage]==da_DK||[installLanguage]==de_DE||[installLanguage]==en_GB||[installLanguage]==en_US||[installLanguage]==es_ES||[installLanguage]==es_MX||[installLanguage]==fi_FI||[installLanguage]==fr_CA||[installLanguage]==fr_FR||[installLanguage]==hu_HU||[installLanguage]==it_IT||[installLanguage]==nb_NO||[installLanguage]==nl_NL||[installLanguage]==pl_PL||[installLanguage]==pt_BR||[installLanguage]==ru_RU||[installLanguage]==sv_SE||[installLanguage]==tr_TR||[installLanguage]==uk_UA";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ConditionParser::new(tokens);
        let expr = parser.parse().unwrap();

        let input = "[OSProcessorFamily]==64-bit&&[OSVersion]<10.14 &&[OSVersion]>=10.13";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ConditionParser::new(tokens);
        let expr = parser.parse().unwrap();
    }

    #[test]
    fn test_eval() {
        let input = "[installLanguage]==cs_CZ||[installLanguage]==da_DK||[installLanguage]==de_DE||[installLanguage]==en_GB||[installLanguage]==en_US||[installLanguage]==es_ES||[installLanguage]==es_MX||[installLanguage]==fi_FI||[installLanguage]==fr_CA||[installLanguage]==fr_FR||[installLanguage]==hu_HU||[installLanguage]==it_IT||[installLanguage]==nb_NO||[installLanguage]==nl_NL||[installLanguage]==pl_PL||[installLanguage]==pt_BR||[installLanguage]==ru_RU||[installLanguage]==sv_SE||[installLanguage]==tr_TR||[installLanguage]==uk_UA";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ConditionParser::new(tokens);
        let expr1 = parser.parse().unwrap();

        let input = "[OSProcessorFamily]==64-bit&&[OSVersion]<10.14 &&[OSVersion]>=10.13&&[installLanguage]==en_US";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ConditionParser::new(tokens);
        let expr2 = parser.parse().unwrap();

        // let vars = strmap! {
        //     "OSProcessorFamily" => EvalValue::String("64-bit"),
        //     "OSVersion" => "10.13.5",
        //     "installLanguage" => "en_US",
        // };

        let mut vars = HashMap::new();

        vars.insert(
            "OSProcessorFamily".to_string(),
            EvalValue::String("64-bit".to_string()),
        );
        vars.insert(
            "OSVersion".to_string(),
            EvalValue::Version("10.13.5".to_string()),
        );
        vars.insert(
            "installLanguage".to_string(),
            EvalValue::StringSet(vec!["da_DK".to_string()]),
        );

        let evaluator = ConditionEvaluator::new(vars, true);

        let result1 = evaluator.evaluate(&expr1).unwrap();
        let result2 = evaluator.evaluate(&expr2).unwrap();

        println!("{} {}", result1, result2);
    }
}
