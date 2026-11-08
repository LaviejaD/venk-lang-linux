#[derive(Debug, Clone)]
pub struct Program { pub items: Vec<Item> }

#[derive(Debug, Clone)]
pub enum Item {
    Vipc { name: String, version: Option<String>, meta: Option<Vec<(String, Expr)>> },
    Vip(Vec<crate::lexer::Tok>),
    Import(ImportDecl),
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Impl(ImplDecl),
    Trait(TraitDecl),
    TypeAlias { name: String, ty: Type },
    Const { name: String, ty: Option<Type>, value: Expr },
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub names: Option<Vec<String>>,
    pub module: Vec<String>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub public: bool,
    pub is_async: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: FnBody,
}

#[derive(Debug, Clone)]
pub enum FnBody { Expr(Expr), Block(Block), None }

#[derive(Debug, Clone)]
pub struct Param { pub mutable: bool, pub name: String, pub ty: Option<Type> }

#[derive(Debug, Clone)]
pub struct Block { pub stmts: Vec<Stmt> }

#[derive(Debug, Clone)]
pub struct StructDecl { pub name: String, pub fields: Vec<Field> }

#[derive(Debug, Clone)]
pub struct Field { pub public: bool, pub name: String, pub ty: Type }

#[derive(Debug, Clone)]
pub struct EnumDecl { pub name: String, pub variants: Vec<Variant> }

#[derive(Debug, Clone)]
pub struct Variant { pub name: String, pub fields: VariantFields }

#[derive(Debug, Clone)]
pub enum VariantFields { Unit, Tuple(Vec<Type>), Record(Vec<Field>) }

#[derive(Debug, Clone)]
pub struct ImplDecl { pub ty: Type, pub methods: Vec<FnDecl> }

#[derive(Debug, Clone)]
pub struct TraitDecl { pub name: String, pub methods: Vec<FnDecl> }

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { mutable: bool, name: String, ty: Option<Type>, value: Option<Expr> },
    Expr { expr: Expr, semi: bool },
    Return(Option<Expr>),
    While { cond: Expr, body: Block },
    Loop(Block),
    For { pattern: Pattern, iter: Expr, body: Block },
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64), Float(f64), Str(String), Char(char), Bool(bool), Null,
    Ident(String),
    Path(Vec<String>),
    Array(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    StructInit(Vec<String>, Vec<(String, Expr)>), // ДОБАВЛЕНО
    Tuple(Vec<Expr>),
    Lambda { params: Vec<Param>, body: Box<Expr>, is_async: bool },
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Assign { target: Box<Expr>, op: AssignOp, value: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Method { recv: Box<Expr>, name: String, args: Vec<Expr> },
    Field { recv: Box<Expr>, name: String },
    OptField { recv: Box<Expr>, name: String },
    Index { recv: Box<Expr>, index: Box<Expr> },
    If { cond: Box<Expr>, then: Block, elifs: Vec<(Expr, Block)>, els: Option<Block> },
    Match { scrutinee: Box<Expr>, arms: Vec<Arm> },
    Block(Block),
    Try { body: Block, catches: Vec<Catch> },
    Propagate(Box<Expr>),
    Cast { expr: Box<Expr>, ty: Type },
    IsType { expr: Box<Expr>, ty: Type },
    Sizeof(Type),
}

#[derive(Debug, Clone)]
pub struct Arm { pub pattern: Pattern, pub guard: Option<Expr>, pub body: Expr }

#[derive(Debug, Clone)]
pub struct Catch { pub binding: Option<String>, pub body: Block }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp { Neg, Not, BitNot }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem, Pow,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Pipe, Default,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignOp { Set, Add, Sub, Mul, Div, Rem }

#[derive(Debug, Clone)]
pub enum Pattern {
    Wild,
    Lit(Expr),
    Binding(String),
    Tuple(Vec<Pattern>),
    Array(Vec<Pattern>),
    Record(Vec<(String, Pattern)>),
    Enum { path: Vec<String>, args: Vec<Pattern> },
    Range { start: Box<Pattern>, end: Box<Pattern>, inclusive: bool },
}

#[derive(Debug, Clone)]
pub enum Type {
    Int, Float, Bool, Char, String, Null, Any, SelfTy,
    Array(Box<Type>),
    Record(Vec<(String, Type)>),
    Tuple(Vec<Type>),
    Fn(Vec<Type>, Box<Type>),
    Option(Box<Type>),
    Named(Vec<String>, Vec<Type>),
}