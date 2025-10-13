//!
//! HyperDrive Install Condition DSL Evaluator
//!

use std::collections::HashMap;
use std::str::FromStr;

#[derive(Eq, PartialEq)]
struct Identifier(String);

#[derive(Eq, PartialEq)]
struct Value(String);

#[derive(Copy, Clone)]
enum Operation {
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

    fn read_identifier(&mut self) -> Result<ConditionToken, usize> {
        // precond: current pos is on '['

        if let Some(c) = self.peek(0) {
            if c != '[' {
                return Err(self.pos);
            }
            self.advance(1);
        } else {
            return Err(self.pos);
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

    fn read_operator(&mut self) -> Result<ConditionToken, usize> {
        let first = self.peek(0).ok_or(self.pos)?;

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
            _ => return Err(self.pos),
        };

        Ok(ConditionToken::Op(operation))
    }

    fn read_value(&mut self) -> Result<ConditionToken, usize> {
        const ILLEGAL: &str = "><=|&[";

        if let Some(c) = self.peek(0) {
            if ILLEGAL.contains(c) {
                return Err(self.pos);
            }
        } else {
            return Err(self.pos);
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

    fn next_token(&mut self) -> Result<ConditionToken, usize> {
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

        Err(self.pos)
    }

    pub fn tokenize(&mut self) -> Result<Vec<ConditionToken>, usize> {
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

    fn parse_or(&mut self) -> Result<ExpressionNode, String> {
        let mut left = self.parse_and()?;

        while let ConditionToken::Op(Operation::Or) = self.current_token() {
            self.advance();
            let right = self.parse_and()?;
            left = ExpressionNode::Or(Box::new(left), Box::new(right))
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ExpressionNode, String> {
        let mut left = self.parse_comparison()?;

        while let ConditionToken::Op(Operation::And) = self.current_token() {
            self.advance();
            let right = self.parse_comparison()?;
            left = ExpressionNode::And(Box::new(left), Box::new(right))
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ExpressionNode, String> {
        let token = self.current_token().clone();

        let ident = match token {
            ConditionToken::Ident(s) => s.clone(),
            _ => return Err("Expected identifier".to_string()),
        };
        self.advance();

        let op_token = self.current_token().clone();
        let operation = match op_token {
            ConditionToken::Op(op) => op.clone(),
            _ => return Err("Expected operator".to_string()),
        };
        self.advance();

        let val_token = self.current_token().clone();
        let v = match val_token {
            ConditionToken::Val(s) => s.clone(),
            _ => return Err("Expected value".to_string()),
        };
        self.advance();

        if !operation.is_cmp() {
            return Err("Invalid operator".to_string());
        }

        let expr = if operation.is_ineq() {
            ExpressionNode::Inequality(Identifier(ident), operation, Value(v.trim().into()))
        } else {
            ExpressionNode::Equality(Identifier(ident), Value(v.trim().into()))
        };

        Ok(expr)
    }

    pub fn parse(&mut self) -> Result<ExpressionNode, String> {
        self.parse_or()
    }
}

fn compare_vers(a: &str, b: &str, op: &Operation) -> bool {
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

enum EvalValue {
    Version(String),
    String(String),
    Bool(bool),
    Number(i32),
}

impl EvalValue {
    pub fn equals(&self, b: &Self) -> bool {
        match (self, b) {
            (EvalValue::String(a), EvalValue::String(b)) => a == b,
            (EvalValue::Bool(a), EvalValue::Bool(b)) => a == b,
            (EvalValue::Version(a), EvalValue::Version(b)) => a == b,
            (EvalValue::Number(a), EvalValue::Number(b)) => a == b,
            _ => false,
        }
    }

    pub fn compare(&self, op: &Operation, b: &Self) -> Result<bool, String> {
        match (self, b) {
            (EvalValue::Version(a), EvalValue::Version(b)) => Ok(compare_vers(a, b, op)),
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
                _ => Err("Strings only support equality".to_string()),
            },
            (EvalValue::Bool(a), EvalValue::Bool(b)) => match op {
                Operation::Eq => Ok(a == b),
                _ => Err("Bools only support equality".to_string()),
            },
            _ => Err("Type mismatch in comparison".to_string()),
        }
    }

    pub fn coerce_str(&self, value: &str) -> Self {
        match self {
            EvalValue::Version(_) => EvalValue::Version(value.to_string()),
            EvalValue::String(_) => EvalValue::String(value.to_string()),
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
    pub fn new(vars: HashMap<String, String>, strict_mode: bool) -> Self {
        let mut vs = HashMap::new();
        for (ident, value) in vars {
            // Known variables:
            //  installLanguage: String
            //  OSVersion: Version
            //  OSProcessorFamily: String
            //  OSArchitectire: String
            //  IsEnterpriseDeployment: Bool

            let val = match ident.as_str() {
                "installLanguage" | "OSProcessorFamily" | "OSArchitecture" => {
                    EvalValue::String(value)
                }
                "OSVersion" => EvalValue::Version(value),
                "IsEnterpriseDeployment" => EvalValue::Bool(value == "true"),
                _ => EvalValue::String(value),
            };

            vs.insert(ident, val);
        }

        Self {
            variables: vs,
            strict: strict_mode,
        }
    }

    fn get_var(&self, ident: &str) -> Option<&EvalValue> {
        self.variables.get(&ident.to_string())
    }

    pub fn evaluate(&self, expression: &ExpressionNode) -> Result<bool, String> {
        match expression {
            ExpressionNode::And(left, right) => Ok(self.evaluate(left)? && self.evaluate(right)?),
            ExpressionNode::Or(left, right) => Ok(self.evaluate(left)? || self.evaluate(right)?),
            ExpressionNode::Equality(ident, value) => {
                if let Some(var) = self.get_var(&ident.0) {
                    let b = var.coerce_str(&value.0);
                    Ok(var.equals(&b))
                } else if !self.strict {
                    Ok(true)
                } else {
                    Err(format!("Undeclared identifier: {}", ident.0))
                }
            }
            ExpressionNode::Inequality(ident, op, val) => {
                if let Some(var) = self.get_var(&ident.0) {
                    let b = var.coerce_str(&val.0);
                    var.compare(op, &b)
                } else if !self.strict {
                    Ok(true)
                } else {
                    Err(format!("Undeclared identifier: {}", ident.0))
                }
            }
        }
    }
}

pub fn parse_condition(expr: &str) -> Result<ExpressionNode, String> {
    let mut lexer = ConditionLexer::new(expr);
    let tokens = lexer
        .tokenize()
        .map_err(|f| format!("Lex error: pos {}", f))?;

    let mut parser = ConditionParser::new(tokens);

    parser.parse()
}

mod tests {
    use crate::hyperdrive::remote::condition::{
        ConditionEvaluator, ConditionLexer, ConditionParser,
    };
    use crate::strmap;
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

        let input = "[OSProcessorFamily]==64-bit&&[OSVersion]<10.14 &&[OSVersion]>=10.13";
        let mut lexer = ConditionLexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = ConditionParser::new(tokens);
        let expr2 = parser.parse().unwrap();

        let vars = strmap! {
            "OSProcessorFamily" => "64-bit",
            "OSVersion" => "10.13.5",
            "installLanguage" => "en_US",
        };
        let evaluator = ConditionEvaluator::new(vars, true);

        let result1 = evaluator.evaluate(&expr1).unwrap();
        let result2 = evaluator.evaluate(&expr2).unwrap();

        println!("{} {}", result1, result2);
    }
}
