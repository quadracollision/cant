use std::fmt;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Identifier(String),
    Self_,  // New: for "self" keyword
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
    // Remove HitsThreshold variant
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
    Hits,  // New: for "ball1 hits self X" syntax
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Minus,
    Not,
}

#[derive(Debug, Clone)]
pub enum SpeedModification {
    Absolute(f64),    // set speed ball1 50
    Relative(f64),    // set speed ball1 +3 or set speed ball1 -0.3
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
    SetSpeed {
        object_name: String,
        speed: SpeedModification,
    },
    Label {
        object_name: String,
        arguments: Vec<Expr>,
        text: String,
    },
    Script {
        object_name: String,
        arguments: Vec<Expr>,
    },
    Play,   // New: simple play command
    Pause,  // New: pause command
    Stop,   // New: stop command to restore pre-play state
    Verbose, // New: verbose command to toggle debug output
    ClearBalls,   // New: clear all balls command
    ClearSquares, // New: clear all squares command
    Destroy {     // New: destroy command
        object_type: String,
        arguments: Vec<Expr>,
    },
    Run {         // New: run script file command
        script_name: String,
    },
    Slice {       // New: slice array command
        sequence: Vec<f64>, // The sequence of marker numbers
    },
    Waveform {    // New: waveform editor command
        target: Option<String>, // Optional audio file path or ball reference
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
            Expr::Self_ => write!(f, "self"),
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
            // Remove this entire HitsThreshold match arm (lines 195-197)
        }
    }
}