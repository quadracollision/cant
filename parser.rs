use crate::lexer::{Token, TokenType};
use crate::ast::*;
use std::fmt;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            // Skip newlines at top level
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }
            
            statements.push(self.statement()?);
        }
        
        Ok(Program { statements })
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(&TokenType::Let) {
            self.let_statement()
        } else if self.match_token(&TokenType::If) {
            self.if_statement()
        } else if self.match_token(&TokenType::While) {
            self.while_statement()
        } else if self.match_token(&TokenType::Function) {
            self.function_statement()
        } else if self.match_token(&TokenType::Return) {
            self.return_statement()
        } else if self.match_token(&TokenType::Set) {
            self.set_statement()
        } else if self.match_token(&TokenType::Play) {
            self.play_statement()
        } else if self.match_token(&TokenType::Pause) {
            self.pause_statement()
        } else if self.match_token(&TokenType::Stop) {
            self.stop_statement()
        } else if self.match_token(&TokenType::Clear) {
            self.clear_statement()
        } else if self.match_token(&TokenType::LeftBrace) {
            Ok(Stmt::Block(self.block()?))
        } else {
            self.expression_statement()
        }
    }

    fn play_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Play)
    }

    fn pause_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Pause)
    }

    fn set_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(&TokenType::Direction, "Expected 'direction' after 'set'")?;
        
        let object_name = if let TokenType::Identifier(name) = &self.peek().token_type {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column));
        };
        
        let direction = match &self.peek().token_type {
            TokenType::Left => { self.advance(); DirectionValue::Left },
            TokenType::Right => { self.advance(); DirectionValue::Right },
            TokenType::Up => { self.advance(); DirectionValue::Up },
            TokenType::Down => { self.advance(); DirectionValue::Down },
            TokenType::UpLeft => { self.advance(); DirectionValue::UpLeft },
            TokenType::UpRight => { self.advance(); DirectionValue::UpRight },
            TokenType::DownLeft => { self.advance(); DirectionValue::DownLeft },
            TokenType::DownRight => { self.advance(); DirectionValue::DownRight },
            _ => return Err(ParseError::UnexpectedToken(self.peek().clone())),
        };
        
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::SetDirection { object_name, direction })
    }

    fn let_statement(&mut self) -> Result<Stmt, ParseError> {
        let name = if let TokenType::Identifier(name) = &self.peek().token_type {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column));
        };

        let initializer = if self.match_token(&TokenType::Assign) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Let { name, initializer })
    }

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(&TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RightParen, "Expected ')' after if condition")?;
        
        let then_branch = Box::new(self.statement()?);
        let else_branch = if self.match_token(&TokenType::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        
        Ok(Stmt::If { condition, then_branch, else_branch })
    }

    fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(&TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RightParen, "Expected ')' after while condition")?;
        
        let body = Box::new(self.statement()?);
        Ok(Stmt::While { condition, body })
    }

    fn function_statement(&mut self) -> Result<Stmt, ParseError> {
        let name = if let TokenType::Identifier(name) = &self.peek().token_type {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column));
        };

        self.consume(&TokenType::LeftParen, "Expected '(' after function name")?;
        
        let mut parameters = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                if let TokenType::Identifier(param) = &self.peek().token_type {
                    parameters.push(param.clone());
                    self.advance();
                } else {
                    return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column));
                }
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after parameters")?;
        self.consume(&TokenType::LeftBrace, "Expected '{' before function body")?;
        
        let body = Box::new(Stmt::Block(self.block()?));
        
        Ok(Stmt::Function { name, parameters, body })
    }

    fn return_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = if self.check(&TokenType::Semicolon) || self.check(&TokenType::Newline) {
            None
        } else {
            Some(self.expression()?)
        };
        
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Return(value))
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }
            statements.push(self.statement()?);
        }
        
        self.consume(&TokenType::RightBrace, "Expected '}' after block")?;
        Ok(statements)
    }

    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression()?;
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Expression(expr))
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.or()?;
        
        if self.match_token(&TokenType::Assign) {
            let value = self.assignment()?;
            if let Expr::Identifier(name) = expr {
                return Ok(Expr::Assignment {
                    name,
                    value: Box::new(value),
                });
            }
            return Err(ParseError::InvalidAssignmentTarget(self.previous().line, self.previous().column));
        }
        
        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        self.and()
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;
        
        while self.match_tokens(&[TokenType::NotEqual, TokenType::Equal]) {
            let operator = match self.previous().token_type {
                TokenType::Equal => BinaryOp::Equal,
                TokenType::NotEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;
        
        while self.match_tokens(&[TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual]) {
            let operator = match self.previous().token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;
        
        while self.match_tokens(&[TokenType::Minus, TokenType::Plus]) {
            let operator = match self.previous().token_type {
                TokenType::Minus => BinaryOp::Subtract,
                TokenType::Plus => BinaryOp::Add,
                _ => unreachable!(),
            };
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;
        
        while self.match_tokens(&[TokenType::Divide, TokenType::Multiply]) {
            let operator = match self.previous().token_type {
                TokenType::Divide => BinaryOp::Divide,
                TokenType::Multiply => BinaryOp::Multiply,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_tokens(&[TokenType::Minus]) {
            let operator = UnaryOp::Minus;
            let right = self.unary()?;
            return Ok(Expr::Unary {
                operator,
                operand: Box::new(right),
            });
        }
        
        self.call()
    }

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        
        while self.match_token(&TokenType::LeftParen) {
            expr = self.finish_call(expr)?;
        }
        
        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let mut arguments = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;
        
        Ok(Expr::Call {
            callee: Box::new(callee),
            arguments,
        })
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        match &self.peek().token_type {
            TokenType::Identifier(name) => {
                // Check for special 'create' syntax
                if name == "create" {
                    self.advance(); // consume 'create'
                    
                    // Expect object type identifier
                    if let TokenType::Identifier(object_type) = &self.peek().token_type {
                        let object_type = object_type.clone();
                        self.advance(); // consume object type
                        
                        // Check for optional parentheses
                        let arguments = if self.check(&TokenType::LeftParen) {
                            self.advance(); // consume '('
                            let mut args = Vec::new();
                            
                            if !self.check(&TokenType::RightParen) {
                                loop {
                                    args.push(self.expression()?);
                                    if !self.match_token(&TokenType::Comma) {
                                        break;
                                    }
                                }
                            }
                            
                            self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;
                            args
                        } else {
                            Vec::new() // No parentheses = no arguments
                        };
                        
                        return Ok(Expr::CreateCall { object_type, arguments });
                    } else {
                        return Err(ParseError::Expected {
                            expected: "object type".to_string(),
                            found: self.peek().clone(),
                            message: "Expected object type after 'create'".to_string(),
                        });
                    }
                }
                
                // Regular identifier
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier(name))
            },
            TokenType::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Number(n))
            },
            TokenType::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s))
            },
            TokenType::LeftParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(&TokenType::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            },
            _ => Err(ParseError::UnexpectedToken(self.peek().clone())),
        }
    }

    // Helper methods
    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_tokens(&mut self, types: &[TokenType]) -> bool {
        for token_type in types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Result<&Token, ParseError> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(ParseError::Expected {
                expected: format!("{:?}", token_type),
                found: self.peek().clone(),
                message: message.to_string(),
            })
        }
    }

    fn consume_newline_or_semicolon(&mut self) -> Result<(), ParseError> {
        if self.check(&TokenType::Semicolon) || self.check(&TokenType::Newline) || self.is_at_end() {
            if !self.is_at_end() {
                self.advance();
            }
            Ok(())
        } else {
            Err(ParseError::Expected {
                expected: "newline or semicolon".to_string(),
                found: self.peek().clone(),
                message: "Expected end of statement".to_string(),
            })
        }
    }

    fn stop_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Stop)
    }

    fn clear_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(&TokenType::Balls) {
            self.consume_newline_or_semicolon()?;
            Ok(Stmt::ClearBalls)
        } else if self.match_token(&TokenType::Squares) {
            self.consume_newline_or_semicolon()?;
            Ok(Stmt::ClearSquares)
        } else {
            Err(ParseError::Expected {
                expected: "'balls' or 'squares'".to_string(),
                found: self.peek().clone(),
                message: "Expected 'balls' or 'squares' after 'clear'".to_string(),
            })
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(Token),
    ExpectedIdentifier(usize, usize),
    InvalidAssignmentTarget(usize, usize),
    Expected {
        expected: String,
        found: Token,
        message: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken(token) => {
                write!(f, "Unexpected token {:?} at line {}, column {}", 
                       token.token_type, token.line, token.column)
            },
            ParseError::ExpectedIdentifier(line, col) => {
                write!(f, "Expected identifier at line {}, column {}", line, col)
            },
            ParseError::InvalidAssignmentTarget(line, col) => {
                write!(f, "Invalid assignment target at line {}, column {}", line, col)
            },
            ParseError::Expected { expected, found, message } => {
                write!(f, "{}: expected {} but found {:?} at line {}, column {}", 
                       message, expected, found.token_type, found.line, found.column)
            },
        }
    }
}

impl std::error::Error for ParseError {}