use std::fmt;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Identifier(String),
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    // New: Special syntax for create commands
    CreateCall {
        object_type: String,
        arguments: Vec<Expr>,
    },
    Assignment {
        name: String,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Minus,
    Not,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(Expr),
    Let {
        name: String,
        initializer: Option<Expr>,
    },
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
    Block(Vec<Stmt>),
    Function {
        name: String,
        parameters: Vec<String>,
        body: Box<Stmt>,
    },
    Return(Option<Expr>),
    SetDirection {
        object_name: String,
        direction: DirectionValue,
    },
    SetColor {
        object_name: String,
        color: ColorValue,
    },
    Label {
        object_name: String,
        arguments: Vec<Expr>,
        text: String,
    },
    Play,   // New: simple play command
    Pause,  // New: pause command
    Stop,   // New: stop command to restore pre-play state
    ClearBalls,   // New: clear all balls command
    ClearSquares, // New: clear all squares command
    Destroy {     // New: destroy command
        object_type: String,
        arguments: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum DirectionValue {
    Left,
    Right,
    Up,
    Down,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

#[derive(Debug, Clone)]
pub enum ColorValue {
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
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::String(s) => write!(f, "\"{}\"", s),
            Expr::Identifier(name) => write!(f, "{}", name),
            Expr::Binary { left, operator, right } => {
                write!(f, "({} {:?} {})", left, operator, right)
            },
            Expr::Unary { operator, operand } => {
                write!(f, "({:?} {})", operator, operand)
            },
            Expr::Call { callee, arguments } => {
                write!(f, "{}(", callee)?;
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            },
            Expr::CreateCall { object_type, arguments } => {
                write!(f, "create {}(", object_type)?;
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            },
            Expr::Assignment { name, value } => {
                write!(f, "{} = {}", name, value)
            },
        }
    }
}