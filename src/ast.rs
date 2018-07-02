/*
 * Copyright 2017 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::cell::RefCell;
use std::rc::Rc;

pub use lex::Span;

/// Symbol represents a resolved lexical symbol.
/// E.g. in
///   let x = 3;
///   x
/// both 'x' will refer to the same Symbol.
#[derive(Debug)]
pub struct Symbol {
    pub name: String,
    /// False if the symbol's name is significant and cannot be changed, e.g. 'arguments'.
    pub renameable: bool,
    pub write: bool,
    pub read: bool,
}

pub type RefSym = Rc<RefCell<Symbol>>;

impl Symbol {
    pub fn new<S: Into<String>>(name: S) -> RefSym {
        Rc::new(RefCell::new(Symbol {
            name: String::from(name.into()),
            renameable: true,
            read: false,
            write: false,
        }))
    }
}

/// Scope is a single lexical scope: a collection symbols.
#[derive(Debug)]
pub struct Scope {
    pub bindings: Vec<RefSym>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            bindings: Vec::new(),
        }
    }

    /// resolve looks up a symbol by name.
    pub fn resolve(&self, sym: &RefSym) -> Option<RefSym> {
        let name = &sym.borrow().name;
        self.bindings
            .iter()
            .find(|s| *s.borrow().name == *name)
            .map(|t| t.clone())
    }
}

#[derive(Debug)]
pub enum Expr {
    // The parse of "()", which is used only in parsing arrow functions,
    // and which should not make it out of the parsing layer.
    EmptyParens,

    // 12.2 Primary Expression
    This,
    Ident(RefSym),
    Null,
    Undefined, // Note: not part of the grammar, hmm.
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Rc<ExprNode>>),
    // The parse of "...a", which can only occur in arrow functions and
    // in array literals.
    Spread(Rc<ExprNode>),
    Object(Box<Object>),
    Function(Box<Function>),
    Class(Box<Class>),
    ArrowFunction(Box<ArrowFunction>),
    Regex(Box<Regex>),
    Template(Box<Template>),

    // 12.3 Left-Hand-Side Expressions
    Index(Rc<ExprNode>, Rc<ExprNode>),
    Field(Rc<ExprNode>, String),
    New(Rc<ExprNode>),
    Call(Box<Call>),

    // Various other operators.
    Unary(UnOp, Rc<ExprNode>),
    Binary(Box<Binary>),
    TypeOf(Rc<ExprNode>),
    Ternary(Box<Ternary>),
    Assign(Rc<ExprNode>, Rc<ExprNode>),
}

impl Expr {
    pub fn kind(&self) -> &'static str {
        match *self {
            Expr::EmptyParens => "empty parens",
            Expr::This => "this",
            Expr::Ident(_) => "ident",
            Expr::Null => "null",
            Expr::Undefined => "undefined",
            Expr::Bool(_) => "bool",
            Expr::Number(_) => "number",
            Expr::String(_) => "string",
            Expr::Array(_) => "array",
            Expr::Spread(_) => "spread",
            Expr::Object(_) => "object",
            Expr::Function(_) => "function",
            Expr::Class(_) => "class",
            Expr::ArrowFunction(_) => "arrow",
            Expr::Regex(_) => "regex",
            Expr::Template(_) => "template",
            Expr::Index(_, _) => "index",
            Expr::Field(_, _) => "field",
            Expr::New(_) => "new",
            Expr::Call(_) => "call",
            Expr::Unary(_, _) => "unary",
            Expr::Binary(_) => "binary",
            Expr::TypeOf(_) => "typeof",
            Expr::Ternary(_) => "ternary",
            Expr::Assign(_, _) => "assign",
        }
    }
}

#[derive(Debug)]
pub struct ExprNode {
    pub span: Span,
    pub expr: Expr,
}
impl ExprNode {
    pub fn new(span: Span, expr: Expr) -> Rc<ExprNode> {
        Rc::new(ExprNode {
            span: span,
            expr: expr,
        })
    }
}

/// Object literal.
#[derive(Debug)]
pub struct Object {
    pub props: Vec<Property>,
}

// Property forms:
// 1) a   (short for a: a)
// 2) a: b
// 3) 'a': b
// 4) 0: b  (numeric key)
// 5) [a]: b
// 6) a(b) {}
// 7) get/set a(b) {}
// 8) a=b (used to cover alternative syntax for bindings)

#[derive(Debug,Clone)]
pub enum PropertyName {
    String(String),
    Number(f64),
    Computed(Rc<ExprNode>),
}

#[derive(Debug)]
pub struct Property {
    pub name: PropertyName,
    pub value: Rc<ExprNode>,
}

#[derive(Debug)]
pub struct ObjectBindingPattern {
    pub props: Vec<(PropertyName, BindingElement)>,
}

#[derive(Debug)]
pub struct ArrayBindingPattern {
    pub elems: Vec<BindingElement>,
}

#[derive(Debug)]
pub enum BindingPattern {
    Name(RefSym),
    Object(ObjectBindingPattern),
    Array(ArrayBindingPattern),
}

impl BindingPattern {
    pub fn is_name(&self) -> bool {
        match *self {
            BindingPattern::Name(_) => true,
            _ => false,
        }
    }
}

pub type BindingElement = (BindingPattern, Option<Rc<ExprNode>>);

/// Attributes shared by functions and methods.
#[derive(Debug)]
pub struct Func {
    pub scope: Scope,
    pub async: bool,
    pub params: Vec<BindingElement>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Function {
    pub name: Option<RefSym>,
    pub func: Func,
}

#[derive(Debug)]
pub struct Method {
    pub is_static: bool,
    pub name: PropertyName,
    pub func: Func,
}

#[derive(Debug)]
pub enum ArrowBody {
    Expr(Rc<ExprNode>),
    Stmts(Vec<Stmt>),
}

#[derive(Debug)]
pub struct ArrowFunction {
    pub params: Vec<BindingElement>,
    pub body: ArrowBody,
}

#[derive(Debug)]
pub struct Class {
    pub name: Option<RefSym>,
    pub extends: Option<Rc<ExprNode>>,
    pub methods: Vec<Method>,
}

#[derive(Debug)]
pub struct Regex {
    // TODO: parse into body and flags?
    pub literal: String,
}

#[derive(Debug)]
pub struct Template {
    // TODO: parse into body and flags?
    pub literal: String,
}

#[derive(Debug)]
pub struct Call {
    pub func: Rc<ExprNode>,
    pub args: Vec<Rc<ExprNode>>,
}

pub use ops::BinOp;
pub use ops::UnOp;

#[derive(Debug)]
pub struct Binary {
    pub op: BinOp,
    pub lhs: Rc<ExprNode>,
    pub rhs: Rc<ExprNode>,
}

#[derive(Debug)]
pub struct Ternary {
    pub condition: Rc<ExprNode>,
    pub iftrue: Rc<ExprNode>,
    pub iffalse: Rc<ExprNode>,
}

#[derive(Debug)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Var(Box<VarDecls>),
    Empty,
    Expr(Rc<ExprNode>),
    If(Box<If>),
    While(Box<While>),
    DoWhile(Box<While>),
    For(Box<For>),
    ForInOf(Box<ForInOf>),
    Switch(Box<Switch>),
    Continue(Option<String>),
    Break(Option<String>),
    Return(Option<Rc<ExprNode>>),
    Label(Box<Label>),
    Throw(Rc<ExprNode>),
    Try(Box<Try>),

    Function(Box<Function>),
    Class(Box<Class>),
}

impl Stmt {
    pub fn kind(&self) -> &'static str {
        match *self {
            Stmt::Block(_) => "block",
            Stmt::Var(_) => "var",
            Stmt::Empty => "empty",
            Stmt::Expr(_) => "expr",
            Stmt::If(_) => "if",
            Stmt::While(_) => "while",
            Stmt::DoWhile(_) => "dowhile",
            Stmt::For(_) => "for",
            Stmt::ForInOf(_) => "for-in",
            Stmt::Switch(_) => "switch",
            Stmt::Continue(_) => "continue",
            Stmt::Break(_) => "break",
            Stmt::Return(_) => "return",
            Stmt::Label(_) => "label",
            Stmt::Throw(_) => "throw",
            Stmt::Try(_) => "try",
            Stmt::Function(_) => "function",
            Stmt::Class(_) => "class",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VarDeclType {
    Var,
    Const,
    Let,
}

impl VarDeclType {
    pub fn to_string(&self) -> &'static str {
        match self {
            &VarDeclType::Var => "var",
            &VarDeclType::Const => "const",
            &VarDeclType::Let => "let",
        }
    }
}

#[derive(Debug)]
pub struct VarDecl {
    pub pattern: BindingPattern,
    pub init: Option<Rc<ExprNode>>,
}

#[derive(Debug)]
pub struct VarDecls {
    pub typ: VarDeclType,
    pub decls: Vec<VarDecl>,
}

#[derive(Debug)]
pub struct If {
    pub cond: Rc<ExprNode>,
    pub iftrue: Stmt,
    pub else_: Option<Stmt>,
}

#[derive(Debug)]
pub struct While {
    pub cond: Rc<ExprNode>,
    pub body: Stmt,
}

#[derive(Debug)]
pub enum ForInit {
    Empty,
    Expr(Rc<ExprNode>),
    Decls(VarDecls),
}

#[derive(Debug)]
pub struct For {
    pub init: ForInit,
    pub cond: Option<Rc<ExprNode>>,
    pub iter: Option<Rc<ExprNode>>,
    pub body: Stmt,
}

#[derive(Debug)]
pub enum InOf {
    In,
    Of,
}

#[derive(Debug)]
pub struct ForInOf {
    pub decl_type: Option<VarDeclType>,
    pub loop_var: BindingPattern,
    pub in_of: InOf,
    pub expr: Rc<ExprNode>,
    pub body: Stmt,
}

#[derive(Debug)]
pub struct Case {
    pub expr: Option<Rc<ExprNode>>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Switch {
    pub expr: Rc<ExprNode>,
    pub cases: Vec<Case>,
}

#[derive(Debug)]
pub struct Label {
    pub label: String,
    pub stmt: Stmt,
}

#[derive(Debug)]
pub struct Try {
    pub block: Stmt,
    pub catch: Option<(BindingPattern, Stmt)>,
    pub finally: Option<Stmt>,
}

#[derive(Debug)]
pub struct Module {
    pub scope: Scope,
    pub stmts: Vec<Stmt>,
}
