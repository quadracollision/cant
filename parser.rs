use crate::lexer::{Token, TokenType};
use crate::ast::{Expr, Stmt, BinaryOp, UnaryOp, DirectionValue, ColorValue, SpeedModification, Program};
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
        } else if self.match_token(&TokenType::Create) {
            self.create_statement()
        } else if self.match_token(&TokenType::Destroy) {
            self.destroy_statement()
        } else if self.match_token(&TokenType::Clear) {
            self.clear_statement()
        } else if self.match_token(&TokenType::Label) {
            self.label_statement()
        } else if self.match_token(&TokenType::Play) {
            self.play_statement()
        } else if self.match_token(&TokenType::Pause) {
            self.pause_statement()
        } else if self.match_token(&TokenType::Stop) {
            self.stop_statement()
        } else if self.match_token(&TokenType::Verbose) {
            self.verbose_statement()
        } else if self.match_token(&TokenType::Script) {
            self.script_statement()
        } else if self.match_token(&TokenType::Run) {
            self.run_statement()
        } else {
            self.expression_statement()
        }
    }

    fn destroy_statement(&mut self) -> Result<Stmt, ParseError> {
        // Parse object type (ball, square, etc.)
        let object_type = match &self.peek().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            },
            _ => return Err(ParseError::Expected {
                expected: "object type".to_string(),
                found: self.peek().clone(),
                message: "Expected object type after 'destroy'".to_string(),
            }),
        };
        
        // Parse arguments in parentheses
        self.consume(&TokenType::LeftParen, "Expected '(' after object type")?;
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
        self.consume_newline_or_semicolon()?;
        
        Ok(Stmt::Destroy {
            object_type,
            arguments,
        })
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
        // Check if it's "set direction", "set color", or "set speed"
        if self.check(&TokenType::Direction) {
            self.advance(); // consume 'direction'
            
            // In set_statement() for "set direction"
            let object_name = match &self.peek().token_type {
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                },
                TokenType::Cursor => {
                    self.advance();
                    "cursor".to_string()
                },
                _ => return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column)),
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
        } else if self.check(&TokenType::Color) {
            self.advance(); // consume 'color'
            
            let object_name = match &self.peek().token_type {
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                },
                TokenType::Cursor => {
                    self.advance();
                    "cursor".to_string()
                },
                _ => return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column)),
            };
            
            let color = match &self.peek().token_type {
                TokenType::Red => { self.advance(); ColorValue::Red },
                TokenType::Blue => { self.advance(); ColorValue::Blue },
                TokenType::Green => { self.advance(); ColorValue::Green },
                TokenType::Yellow => { self.advance(); ColorValue::Yellow },
                TokenType::Orange => { self.advance(); ColorValue::Orange },
                TokenType::Purple => { self.advance(); ColorValue::Purple },
                TokenType::Pink => { self.advance(); ColorValue::Pink },
                TokenType::Cyan => { self.advance(); ColorValue::Cyan },
                TokenType::Magenta => { self.advance(); ColorValue::Magenta },
                TokenType::White => { self.advance(); ColorValue::White },
                TokenType::Black => { self.advance(); ColorValue::Black },
                TokenType::Gray => { self.advance(); ColorValue::Gray },
                TokenType::Brown => { self.advance(); ColorValue::Brown },
                TokenType::Lime => { self.advance(); ColorValue::Lime },
                _ => return Err(ParseError::UnexpectedToken(self.peek().clone())),
            };
            
            self.consume_newline_or_semicolon()?;
            Ok(Stmt::SetColor { object_name, color })
        } else if self.check(&TokenType::Speed) {
            self.advance(); // consume 'speed'
            
            let object_name = match &self.peek().token_type {
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                },
                TokenType::Cursor => {
                    self.advance();
                    "cursor".to_string()
                },
                _ => return Err(ParseError::ExpectedIdentifier(self.peek().line, self.peek().column)),
            };
            
            // Check for + or - prefix for relative speed changes
            let speed = match &self.peek().token_type {
                TokenType::Plus => {
                    self.advance(); // consume '+'
                    match &self.peek().token_type {
                        TokenType::Number(value) => {
                            let speed_value = *value;
                            self.advance();
                            SpeedModification::Relative(speed_value)
                        },
                        _ => return Err(ParseError::Expected {
                            expected: "number".to_string(),
                            found: self.peek().clone(),
                            message: "Expected a number after '+' for relative speed".to_string(),
                        }),
                    }
                },
                TokenType::Minus => {
                    self.advance(); // consume '-'
                    match &self.peek().token_type {
                        TokenType::Number(value) => {
                            let speed_value = -*value; // negative for subtraction
                            self.advance();
                            SpeedModification::Relative(speed_value)
                        },
                        _ => return Err(ParseError::Expected {
                            expected: "number".to_string(),
                            found: self.peek().clone(),
                            message: "Expected a number after '-' for relative speed".to_string(),
                        }),
                    }
                },
                TokenType::Number(value) => {
                    let speed_value = *value;
                    self.advance();
                    SpeedModification::Absolute(speed_value)
                },
                _ => return Err(ParseError::Expected {
                    expected: "number, '+number', or '-number'".to_string(),
                    found: self.peek().clone(),
                    message: "Expected a number or relative speed change (+/-)".to_string(),
                }),
            };
            
            self.consume_newline_or_semicolon()?;
            Ok(Stmt::SetSpeed { object_name, speed })
        } else {
            Err(ParseError::Expected {
                expected: "'direction', 'color', or 'speed'".to_string(),
                found: self.peek().clone(),
                message: "Expected 'direction', 'color', or 'speed' after 'set'".to_string(),
            })
        }
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
        let condition = self.expression()?; // No parentheses required
        
        // Parse statements until next block initiator or EOF
        let then_branch = Box::new(Stmt::Block(self.parse_implicit_block()?));
        
        let else_branch = if self.match_token(&TokenType::Else) {
            Some(Box::new(Stmt::Block(self.parse_implicit_block()?)))
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

    fn script_statement(&mut self) -> Result<Stmt, ParseError> {
        // Parse object name in parentheses: script(object_name)
        self.consume(&TokenType::LeftParen, "Expected '(' after 'script'")?;
        
        let mut object_name = match &self.peek().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            },
            TokenType::Cursor => {
                self.advance();
                "cursor".to_string()
            },
            _ => return Err(ParseError::Expected {
                expected: "object name or 'cursor'".to_string(),
                found: self.peek().clone(),
                message: "Expected object name or 'cursor' after 'script('".to_string(),
            }),
        };
        
        // Handle dotted identifiers like lib.script_0
        while self.match_token(&TokenType::Dot) {
            if let TokenType::Identifier(name) = &self.peek().token_type {
                object_name.push('.');
                object_name.push_str(name);
                self.advance();
            } else {
                return Err(ParseError::Expected {
                    expected: "identifier after dot".to_string(),
                    found: self.peek().clone(),
                    message: "Expected identifier after '.' in dotted name".to_string(),
                });
            }
        }
        
        // Parse optional arguments (for future extensibility)
        let mut arguments = Vec::new();
        if self.match_token(&TokenType::Comma) {
            if !self.check(&TokenType::RightParen) {
                loop {
                    arguments.push(self.expression()?);
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after script arguments")?;
        self.consume_newline_or_semicolon()?;
        
        Ok(Stmt::Script {
            object_name,
            arguments,
        })
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
        self.consume_newline_or_semicolon()?; // Instead of expecting '{'
        
        // Parse statements until next function/if or EOF
        let body = Box::new(Stmt::Block(self.parse_implicit_block()?));
        
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
        
        while self.match_tokens(&[TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual, TokenType::Hits]) {
            let operator = match self.previous().token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                TokenType::Hits => BinaryOp::Hits,
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
            TokenType::Number(n) => {
                let value = *n;
                self.advance();
                Ok(Expr::Number(value))
            },
            TokenType::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expr::String(value))
            },
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier(name))
            },
            TokenType::Self_ => {
                self.advance();
                Ok(Expr::Self_)
            },
            TokenType::Cursor => {
                self.advance();
                Ok(Expr::Identifier("cursor".to_string()))
            },
            TokenType::LeftParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(&TokenType::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            },
            TokenType::Speed => {
                // Allow 'speed' to be used as a function name
                self.advance();
                Ok(Expr::Identifier("speed".to_string()))
            },
            TokenType::Cursor => {
                self.advance();
                Ok(Expr::Identifier("cursor".to_string()))
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

    fn label_statement(&mut self) -> Result<Stmt, ParseError> {
        // Parse object type or object name
        let object_identifier = match &self.peek().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            },
            TokenType::Cursor => {
                self.advance();
                "cursor".to_string()
            },
            _ => return Err(ParseError::Expected {
                expected: "object identifier".to_string(),
                found: self.peek().clone(),
                message: "Expected object identifier after 'label'".to_string(),
            }),
        };
        
        // Check if we have parentheses (function-style syntax) or direct object name
        let (object_name, arguments) = if self.check(&TokenType::LeftParen) {
            // Function-style syntax: label square(1) text or label square(cursor) text
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
            (object_identifier, args)
        } else {
            // Handle direct cursor usage: "label cursor hello"
            if object_identifier == "cursor" {
                ("cursor".to_string(), Vec::new())
            } else if object_identifier.starts_with("square") {
                // Direct object name syntax: label square1 text
                // Treat the identifier as an object name with ID extracted from it
                // Extract ID from square name (e.g., "square1" -> ID 1)
                let id_str = &object_identifier[6..]; // Remove "square" prefix
                if let Ok(id) = id_str.parse::<f64>() {
                    ("square".to_string(), vec![Expr::Number(id)])
                } else {
                    return Err(ParseError::Expected {
                        expected: "valid square ID".to_string(),
                        found: self.peek().clone(),
                        message: format!("Invalid square identifier: {}", object_identifier),
                    });
                }
            } else {
                return Err(ParseError::Expected {
                    expected: "square identifier or cursor".to_string(),
                    found: self.peek().clone(),
                    message: "Only square objects and cursor are supported for labeling".to_string(),
                });
            }
        };
        
        // Parse the text to label
        let text = match &self.peek().token_type {
            TokenType::Identifier(text) | TokenType::String(text) => {
                let text = text.clone();
                self.advance();
                text
            },
            // Accept color tokens as valid text for labels
            TokenType::Red => { self.advance(); "red".to_string() },
            TokenType::Blue => { self.advance(); "blue".to_string() },
            TokenType::Green => { self.advance(); "green".to_string() },
            TokenType::Yellow => { self.advance(); "yellow".to_string() },
            TokenType::Orange => { self.advance(); "orange".to_string() },
            TokenType::Purple => { self.advance(); "purple".to_string() },
            TokenType::Pink => { self.advance(); "pink".to_string() },
            TokenType::Cyan => { self.advance(); "cyan".to_string() },
            TokenType::Magenta => { self.advance(); "magenta".to_string() },
            TokenType::White => { self.advance(); "white".to_string() },
            TokenType::Black => { self.advance(); "black".to_string() },
            TokenType::Gray => { self.advance(); "gray".to_string() },
            TokenType::Brown => { self.advance(); "brown".to_string() },
            TokenType::Lime => { self.advance(); "lime".to_string() },
            _ => return Err(ParseError::Expected {
                expected: "text".to_string(),
                found: self.peek().clone(),
                message: "Expected text after label arguments".to_string(),
            }),
        };
        
        self.consume_newline_or_semicolon()?;
        
        Ok(Stmt::Label {
            object_name,
            arguments,
            text,
        })
    }

    fn create_statement(&mut self) -> Result<Stmt, ParseError> {
        // Parse object type (ball, square, etc.)
        let object_type = match &self.peek().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            },
            _ => return Err(ParseError::Expected {
                expected: "object type".to_string(),
                found: self.peek().clone(),
                message: "Expected object type after 'create'".to_string(),
            }),
        };
        
        // Parse arguments in parentheses
        self.consume(&TokenType::LeftParen, "Expected '(' after object type")?;
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
        self.consume_newline_or_semicolon()?;
        
        Ok(Stmt::Expression(Expr::CreateCall {
            object_type,
            arguments,
        }))
    }

    fn verbose_statement(&mut self) -> Result<Stmt, ParseError> {
        Ok(Stmt::Verbose)
    }
    
    fn run_statement(&mut self) -> Result<Stmt, ParseError> {
        let script_name = if let TokenType::Identifier(name) = &self.peek().token_type {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(ParseError::Expected {
                expected: "script name".to_string(),
                found: self.peek().clone(),
                message: "Expected script name after 'run'".to_string(),
            });
        };
        
        self.consume_newline_or_semicolon()?;
        Ok(Stmt::Run { script_name })
    }

    fn parse_implicit_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() && !self.is_block_initiator() {
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }
            statements.push(self.statement()?);
        }
        
        Ok(statements)
    }

    fn is_block_initiator(&self) -> bool {
        matches!(self.peek().token_type, 
            TokenType::Function | 
            TokenType::If | 
            TokenType::While |
            TokenType::Else
        )
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