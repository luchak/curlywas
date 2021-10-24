#[derive(Debug, Clone, Copy)]
pub struct Position(pub usize);

#[derive(Debug)]
pub struct Script<'a> {
    pub items: Vec<TopLevelItem<'a>>,
}

#[derive(Debug)]
pub enum TopLevelItem<'a> {
    GlobalVar(GlobalVar<'a>),
    Function(Function<'a>),
}

#[derive(Debug)]
pub struct GlobalVar<'a> {
    pub position: Position,
    pub visibility: Visibility,
    pub name: &'a str,
    pub type_: Type,
}

#[derive(Debug)]
pub struct Function<'a> {
    pub position: Position,
    pub visibility: Visibility,
    pub name: &'a str,
    pub params: Vec<(&'a str, Type)>,
    pub type_: Option<Type>,
    pub body: Block<'a>,
}

#[derive(Debug)]
pub struct Block<'a> {
    pub statements: Vec<Statement<'a>>,
    pub final_expression: Option<Expression<'a>>,
}

#[derive(Debug)]
pub enum Statement<'a> {
    LocalVariable(LocalVariable<'a>),
    Poke {
        mem_location: MemoryLocation<'a>,
        value: Expression<'a>,
    },
    Expression(Expression<'a>),
}

#[derive(Debug)]
pub struct MemoryLocation<'a> {
    pub position: Position,
    pub size: MemSize,
    pub left: Expression<'a>,
    pub right: Expression<'a>,
}

#[derive(Debug)]
pub struct LocalVariable<'a> {
    pub position: Position,
    pub name: &'a str,
    pub type_: Option<Type>,
    pub value: Option<Expression<'a>>,
}

#[derive(Debug)]
pub enum Expression<'a> {
    I32Const(i32),
    Variable {
        position: Position,
        name: &'a str,
    },
    Loop {
        position: Position,
        label: &'a str,
        block: Box<Block<'a>>,
    },
    BranchIf {
        position: Position,
        condition: Box<Expression<'a>>,
        label: &'a str,
    },
    BinOp {
        position: Position,
        op: BinOp,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    LocalTee {
        position: Position,
        name: &'a str,
        value: Box<Expression<'a>>,
    },
}

#[derive(Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSize {
    Byte,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Local,
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
}
