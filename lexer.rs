use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Number(f64),
    String(String),
    Identifier(String),
    
    // Keywords
    Let,
    If,
    Else,
    While,
    For,
    Function,
    Return,
    Set,        // for "set direction" command
    Direction,  // for "direction" keyword
    Color,      // New: for "color" keyword
    Speed,      // New: for "speed" keyword
    Create,     // New: for "create" command
    Play,       // New: for "play" command
    Pause,      // Add this missing token
    Stop,       // New: for "stop" command
    Clear,      // New: for "clear" command
    Destroy,    // New: for "destroy" command
    Label,      // New: for "label" command
    Script,     // New: for "script" command
    Balls,      // New: for "balls" keyword
    Squares,    // New: for "squares" keyword
    Cursor,     // New: for "cursor" keyword
    Self_,      // New: for "self" keyword
    Hits,       // New: for "hits" operator
    Verbose,    // New: for "verbose" command
    Run,        // New: for "run" command
    Slice,      // New: for "slice" command
    Waveform,   // New: for "waveform" command
    
    // Direction keywords
    Left,
    Right,
    Up,
    Down,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
    
    // Color keywords
    Red,
    Blue,
    Green,
    Yellow,
    Orange,
    Purple,
    Pink,
    Cyan,
    Magenta,
    White,
    Black,
    Gray,
    Brown,
    Lime,
    
    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Assign,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Semicolon,
    Dot,
    
    // Special
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(token_type: TokenType, line: usize, column: usize) -> Self {
        Self {
            token_type,
            line,
            column,
        }
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        
        while !self.is_at_end() {
            match self.next_token() {
                Ok(token) => tokens.push(token),
                Err(e) => return Err(e),
            }
        }
        
        tokens.push(Token::new(TokenType::Eof, self.line, self.column));
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexerError> {
        self.skip_whitespace();
        
        if self.is_at_end() {
            return Ok(Token::new(TokenType::Eof, self.line, self.column));
        }
        
        let start_line = self.line;
        let start_column = self.column;
        let c = self.advance();
        
        match c {
            '(' => Ok(Token::new(TokenType::LeftParen, start_line, start_column)),
            ')' => Ok(Token::new(TokenType::RightParen, start_line, start_column)),
            '{' => Ok(Token::new(TokenType::LeftBrace, start_line, start_column)),
            '}' => Ok(Token::new(TokenType::RightBrace, start_line, start_column)),
            '[' => Ok(Token::new(TokenType::LeftBracket, start_line, start_column)),
            ']' => Ok(Token::new(TokenType::RightBracket, start_line, start_column)),
            ',' => Ok(Token::new(TokenType::Comma, start_line, start_column)),
            ';' => Ok(Token::new(TokenType::Semicolon, start_line, start_column)),
            '.' => Ok(Token::new(TokenType::Dot, start_line, start_column)),
            '+' => Ok(Token::new(TokenType::Plus, start_line, start_column)),
            '-' => Ok(Token::new(TokenType::Minus, start_line, start_column)),
            '*' => Ok(Token::new(TokenType::Multiply, start_line, start_column)),
            '/' => Ok(Token::new(TokenType::Divide, start_line, start_column)),
            '=' => {
                if self.match_char('=') {
                    Ok(Token::new(TokenType::Equal, start_line, start_column))
                } else {
                    Ok(Token::new(TokenType::Assign, start_line, start_column))
                }
            },
            '!' => {
                if self.match_char('=') {
                    Ok(Token::new(TokenType::NotEqual, start_line, start_column))
                } else {
                    Err(LexerError::UnexpectedCharacter(c, start_line, start_column))
                }
            },
            '<' => {
                if self.match_char('=') {
                    Ok(Token::new(TokenType::LessEqual, start_line, start_column))
                } else {
                    Ok(Token::new(TokenType::Less, start_line, start_column))
                }
            },
            '>' => {
                if self.match_char('=') {
                    Ok(Token::new(TokenType::GreaterEqual, start_line, start_column))
                } else {
                    Ok(Token::new(TokenType::Greater, start_line, start_column))
                }
            },
            '"' => {
                match self.read_string() {
                    Ok(s) => Ok(Token::new(TokenType::String(s), start_line, start_column)),
                    Err(e) => Err(e),
                }
            },
            '\n' => {
                self.line += 1;
                self.column = 1;
                Ok(Token::new(TokenType::Newline, start_line, start_column))
            },
            c if c.is_ascii_digit() => {
                match self.read_number(c) {
                    Ok(n) => Ok(Token::new(TokenType::Number(n), start_line, start_column)),
                    Err(e) => Err(e),
                }
            },
            c if c.is_ascii_alphabetic() || c == '_' => {
                let identifier = self.read_identifier(c);
                let token_type = self.keyword_or_identifier(identifier);
                Ok(Token::new(token_type, start_line, start_column))
            },
            _ => Err(LexerError::UnexpectedCharacter(c, start_line, start_column)),
        }
    }

    fn advance(&mut self) -> char {
        let c = self.input[self.position];
        self.position += 1;
        self.column += 1;
        c
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.input[self.position] != expected {
            false
        } else {
            self.position += 1;
            self.column += 1;
            true
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                },
                _ => break,
            }
        }
    }

    fn read_string(&mut self) -> Result<String, LexerError> {
        let mut value = String::new();
        let start_line = self.line;
        let start_column = self.column;
        
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }
            value.push(self.advance());
        }
        
        if self.is_at_end() {
            return Err(LexerError::UnterminatedString(start_line, start_column));
        }
        
        // Consume the closing "
        self.advance();
        Ok(value)
    }

    fn read_number(&mut self, first_digit: char) -> Result<f64, LexerError> {
        let mut number_str = String::new();
        number_str.push(first_digit);
        
        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            number_str.push(self.advance());
        }
        
        number_str.parse().map_err(|_| {
            LexerError::InvalidNumber(number_str, self.line, self.column)
        })
    }

    fn read_identifier(&mut self, first_char: char) -> String {
        let mut identifier = String::new();
        identifier.push(first_char);
        
        while !self.is_at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == '_') {
            identifier.push(self.advance());
        }
        
        identifier
    }

    fn keyword_or_identifier(&self, text: String) -> TokenType {
        match text.as_str() {
            "let" => TokenType::Let,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "function" => TokenType::Function,
            "return" => TokenType::Return,
            "set" => TokenType::Set,
            "direction" => TokenType::Direction,
            "color" => TokenType::Color,
            "speed" => TokenType::Speed,
            "create" => TokenType::Create,
            "run" => TokenType::Run,
            "play" | "bang" => TokenType::Play,
            "pause" => TokenType::Pause,
            "stop" => TokenType::Stop,
            "clear" => TokenType::Clear,
            "destroy" => TokenType::Destroy,
            "label" => TokenType::Label,
            "script" => TokenType::Script,
            "balls" => TokenType::Balls,
            "squares" => TokenType::Squares,
            "cursor" => TokenType::Cursor,
            "self" => TokenType::Self_,
            "hits" => TokenType::Hits,
            "verbose" => TokenType::Verbose,
            "slice" => TokenType::Slice,
                "waveform" => TokenType::Waveform,
            "left" => TokenType::Left,
            "right" => TokenType::Right,
            "up" => TokenType::Up,
            "down" => TokenType::Down,
            "up-left" | "left-up" => TokenType::UpLeft,
            "up-right" | "right-up" => TokenType::UpRight,
            "down-left" | "left-down" => TokenType::DownLeft,
            "down-right" | "right-down" => TokenType::DownRight,
            // Color keywords
            "red" => TokenType::Red,
            "blue" => TokenType::Blue,
            "green" => TokenType::Green,
            "yellow" => TokenType::Yellow,
            "orange" => TokenType::Orange,
            "purple" => TokenType::Purple,
            "pink" => TokenType::Pink,
            "cyan" => TokenType::Cyan,
            "magenta" => TokenType::Magenta,
            "white" => TokenType::White,
            "black" => TokenType::Black,
            "gray" => TokenType::Gray,
            "brown" => TokenType::Brown,
            "lime" => TokenType::Lime,
            _ => TokenType::Identifier(text),
        }
    }
}

#[derive(Debug)]
pub enum LexerError {
    UnexpectedCharacter(char, usize, usize),
    UnterminatedString(usize, usize),
    InvalidNumber(String, usize, usize),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexerError::UnexpectedCharacter(ch, line, col) => {
                write!(f, "Unexpected character '{}' at line {}, column {}", ch, line, col)
            },
            LexerError::UnterminatedString(line, col) => {
                write!(f, "Unterminated string at line {}, column {}", line, col)
            },
            LexerError::InvalidNumber(num, line, col) => {
                write!(f, "Invalid number '{}' at line {}, column {}", num, line, col)
            },
        }
    }
}

impl std::error::Error for LexerError {}