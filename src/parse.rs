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

use std;
use std::rc::Rc;
use lex;
use lex::{LexError, Span, Tok, Token};
use ast;
use ast::{Expr, ExprNode, Stmt};

fn todo_span() -> Span {
    Span::new(0, 0)
}

pub struct Parser<'a> {
    pub lexer: lex::Lexer<'a>,
}

#[derive(Debug)]
pub struct ParseError {
    msg: String,
    at: Span,
}

impl ParseError {
    fn from_lexerror(LexError { msg, pos }: LexError) -> ParseError {
        ParseError {
            msg,
            at: Span {
                start: pos,
                end: pos + 1,
            },
        }
    }

    pub fn print(&self, l: &lex::Lexer) {
        let lex::Context {
            line,
            col,
            source_line,
        } = l.scan.context(self.at.start);
        eprintln!("ERROR:{}:{}: {}", line, col, self.msg);
        eprintln!("{}", std::str::from_utf8(source_line).unwrap());
        let mut mark = String::new();
        for _ in 0..col - 1 {
            mark.push(' ');
        }
        for _ in self.at.start..self.at.end {
            mark.push('^');
        }
        eprintln!("{}", mark);
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

fn is_for_in_of_head(expr: &Expr) -> bool {
    match *expr {
        ast::Expr::Binary(ref bin) if bin.op == ast::BinOp::In => true,
        _ => false,
    }
}


fn decl_type_from_tok(tok: Tok) -> ast::VarDeclType {
    match tok {
        Tok::Var => ast::VarDeclType::Var,
        Tok::Let => ast::VarDeclType::Let,
        Tok::Const => ast::VarDeclType::Const,
        _ => unreachable!(),
    }
}

// Given an expression that occurs after a 'var', extract the vars
// that were actually declared.
// E.g. given the input
//     var x = 3, y
//         *      *
// this given the expressions marked *.
fn decls_from_expr(expr: ExprNode, decls: &mut Vec<ast::VarDecl>) -> ParseResult<()> {
    match expr.1 {
        ast::Expr::Ident(name) => {
            decls.push(ast::VarDecl {
                pattern: ast::BindingPattern::Name(name),
                init: None,
            });
        }
        ast::Expr::Assign(lhs, rhs) => match *lhs {
            (_, ast::Expr::Ident(name)) => {
                decls.push(ast::VarDecl {
                    pattern: ast::BindingPattern::Name(name),
                    init: Some(*rhs),
                });
            }
            _ => panic!("decls_from_expr"),
        },
        ast::Expr::Binary(bin) => {
            let bin = *bin;
            if bin.op == ast::BinOp::Comma {
                decls_from_expr(bin.lhs, decls)?;
                decls_from_expr(bin.rhs, decls)?;
            } else {
                return Err(ParseError{
                    msg: "failed to extract decls".into(),
                    at: expr.0,
                });
            }
        }
        _ => {
            return Err(ParseError{
                msg: "failed to extract decls".into(),
                at: expr.0,
            });
        }
    }
    Ok(())
}

// Parsing arrow functions is tricky.  We don't know we're in an
// arrow function until we see the => token, so when we see the
// initial left paren (or the bare identifier) for the param list
// we parse it as an expression initially.  In the JS spec this
// is described as "CoverParenthesizedExpressionAndArrowParameterList".
//
// To handle this, we parse first as an expression, then this function
// attempts to convert that expression into the parameter list of
// an arrow function.  It can fail, in inputs like
//     (x++) => 3
// where we can't convert x++ into a parameter list.
fn arrow_params_from_expr(expr: ExprNode, params: &mut ast::ParameterList) -> ParseResult<()> {
    match expr.1 {
        ast::Expr::EmptyParens => { /* ok, no params */ }
        ast::Expr::Ident(sym) => {
            params.push((ast::BindingPattern::Name(sym), None));
        }
        ast::Expr::Binary(bin) => {
            let bin = *bin;
            if bin.op == ast::BinOp::Comma {
                arrow_params_from_expr(bin.lhs, params)?;
                arrow_params_from_expr(bin.rhs, params)?;
            } else {
                return Err(ParseError {
                    msg: format!("couldn't convert left side of arrow into parameter list"),
                    at: expr.0,
                });
            }
        }
        _ => {
            println!("on: {:?}", expr);
            return Err(ParseError {
                msg: format!("couldn't convert left side of arrow into parameter list"),
                at: expr.0,
            });
        }
    }
    Ok(())
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Parser<'a> {
        Parser {
            lexer: lex::Lexer::new(input),
        }
    }

    fn parse_error<S: Into<String>>(&self, got: Token, expected: S) -> ParseError {
        ParseError {
            msg: format!("expected {}, got {:?}", expected.into(), got.tok),
            at: got.span,
        }
    }

    fn lex_read(&mut self) -> ParseResult<Token> {
        self.lexer.read().map_err(ParseError::from_lexerror)
    }

    fn lex_peek(&mut self) -> ParseResult<Tok> {
        self.lexer.peek().map_err(ParseError::from_lexerror)
    }

    fn expect(&mut self, tok: Tok) -> ParseResult<usize> {
        let token = self.lex_read()?;
        if token.tok == tok {
            Ok(token.span.end)
        } else {
            Err(self.parse_error(token, format!("{:?}", tok)))
        }
    }

    fn array_literal(&mut self, start: usize) -> ParseResult<(Span, Vec<ExprNode>)> {
        let mut elems: Vec<ExprNode> = Vec::new();
        loop {
            match self.lex_peek()? {
                Tok::RSquare => break,
                Tok::Comma => {
                    // elision TODO
                    self.lex_read()?;
                }
                _ => {
                    elems.push(self.expr_prec(1)?);
                    if self.lex_peek()? == Tok::Comma {
                        self.lex_read()?;
                    } else {
                        break;
                    }
                }
            }
        }
        let end = self.expect(Tok::RSquare)?;
        Ok((Span::new(start, end), elems))
    }

    fn object_literal(&mut self) -> ParseResult<ast::Object> {
        let mut props: Vec<ast::Property> = Vec::new();
        loop {
            let (span, name, can_pun) = match self.lex_peek()? {
                Tok::Ident => {
                    let token = self.lex_read()?;
                    (
                        token.span.clone(),
                        ast::PropertyKey::String(self.lexer.text(token)),
                        true,
                    )
                }
                tok if tok.is_kw() => {
                    let token = self.lex_read()?;
                    (
                        token.span.clone(),
                        ast::PropertyKey::String(self.lexer.text(token)),
                        true,
                    )
                }
                Tok::String => {
                    let token = self.lex_read()?;
                    if let lex::TokData::String(s) = token.data {
                        (token.span.clone(), ast::PropertyKey::String(s), false)
                    } else {
                        unreachable!();
                    }
                }
                Tok::Number => {
                    let token = self.lex_read()?;
                    if let lex::TokData::Number(n) = token.data {
                        (token.span, ast::PropertyKey::Number(n), false)
                    } else {
                        unreachable!();
                    }
                }
                _ => break,
            };

            let prop = match self.lex_peek()? {
                Tok::Colon => {
                    // a: b
                    self.lex_read()?;
                    ast::Property {
                        name: name,
                        value: self.expr_prec(3)?,
                    }
                }
                Tok::LParen if can_pun => {
                    // a(...) {}
                    let value = match name {
                        ast::PropertyKey::String(ref s) => {
                            let sym = ast::Symbol::new(s.clone());
                            (
                                span, // TODO: correct span?
                                ast::Expr::Function(Box::new(self.function_from_paren(Some(sym))?)),
                            )
                        }
                        _ => unreachable!(),
                    };
                    ast::Property {
                        name: name,
                        value: value,
                    }
                }
                _ if can_pun => {
                    // Assume it's an object like
                    //   {a, b}
                    // and we hit the comma or right brace,
                    // and then let the below code handle that.
                    let value = match name {
                        ast::PropertyKey::String(ref s) => {
                            (span, ast::Expr::Ident(ast::Symbol::new(s.clone())))
                        }
                        _ => unreachable!(),
                    };
                    ast::Property {
                        name: name,
                        value: value,
                    }
                }
                _ => {
                    let token = self.lex_read()?;
                    return Err(self.parse_error(token, "property value"));
                }
            };
            props.push(prop);

            if self.lex_peek()? != Tok::Comma {
                break;
            }
            self.lex_read()?;
        }
        try!(self.expect(Tok::RBrace));
        Ok(ast::Object { props: props })
    }

    // 12.2 Primary Expression
    // TODO: we need to allow this to fail to handle 'case' clauses in switch properly.
    // Need a primary_expr_opt() that this calls.
    fn primary_expr(&mut self) -> ParseResult<ExprNode> {
        let token = self.lex_read()?;
        Ok(match token.tok {
            Tok::This => (token.span, Expr::This),
            Tok::Ident => {
                let span = token.span.clone();
                let text = self.lexer.text(token);
                (
                    span,
                    match text.as_str() {
                        "null" => Expr::Null,
                        "undefined" => Expr::Undefined,
                        "true" => Expr::Bool(true),
                        "false" => Expr::Bool(false),
                        _ => Expr::Ident(ast::Symbol::new(text)),
                    },
                )
            }
            Tok::Number => {
                if let lex::TokData::Number(n) = token.data {
                    (token.span, Expr::Number(n))
                } else {
                    unreachable!();
                }
            }
            Tok::String => {
                let span = token.span.clone();
                (span, Expr::String(self.lexer.text(token)))
            }
            Tok::LSquare => {
                let (span, arr) = self.array_literal(token.span.start)?;
                (span, Expr::Array(arr))
            }
            Tok::LBrace => (
                todo_span(),
                Expr::Object(Box::new(try!(self.object_literal()))),
            ),
            Tok::Function => (todo_span(), Expr::Function(Box::new(self.function()?))),
            Tok::Class => (todo_span(), Expr::Class(Box::new(self.class()?))),
            Tok::LParen => {
                if self.lex_peek()? == Tok::RParen {
                    let tok = self.lex_read()?;
                    (tok.span, Expr::EmptyParens)
                } else {
                    let r = self.expr()?;
                    self.expect(Tok::RParen)?;
                    r
                }
            }
            Tok::Div => {
                let literal = match self.lexer.read_regex() {
                    Err(err) => panic!(err),
                    Ok(literal) => literal,
                };
                (
                    todo_span(),
                    Expr::Regex(Box::new(ast::Regex {
                        literal: String::from(literal),
                    })),
                )
            }
            _ => {
                return Err(self.parse_error(token, "primary expression"));
            }
        })
    }

    // 14.5 Class Definitions
    fn class(&mut self) -> ParseResult<ast::Class> {
        let name = match self.lex_peek()? {
            Tok::Ident => {
                let token = self.lex_read()?;
                Some(ast::Symbol::new(self.lexer.text(token)))
            }
            _ => None,
        };
        let extends = if self.lex_peek()? == Tok::Extends {
            self.lex_read()?;
            Some(self.expr_prec(/* left-hand side expression */ 18)?)
        } else {
            None
        };
        let mut methods: Vec<ast::Function> = Vec::new();
        self.expect(Tok::LBrace)?;
        loop {
            let token = self.lex_read()?;
            match token.tok {
                Tok::Ident => {
                    let name = Some(ast::Symbol::new(self.lexer.text(token)));
                    methods.push(self.function_from_paren(name)?);
                }
                Tok::Semi => {
                    // Stray extra semis are allowed per spec.
                }
                Tok::RBrace => break,
                _ => {
                    return Err(self.parse_error(token, "class body"));
                }
            }
        }
        Ok(ast::Class {
            name: name,
            extends: extends,
            methods: methods,
        })
    }

    // See BindingPattern and BindingElement in the spec,
    // but note that this also encompasses plain names.
    fn binding_pattern(&mut self) -> ParseResult<ast::BindingPattern> {
        let token = self.lex_read()?;
        Ok(match token.tok {
            Tok::Ident => ast::BindingPattern::Name(ast::Symbol::new(self.lexer.text(token))),
            Tok::LBrace => {
                let mut props: Vec<Rc<ast::Symbol>> = Vec::new();
                loop {
                    // BindingProperty
                    let token = self.lex_read()?;
                    match token.tok {
                        Tok::Ident => {
                            props.push(ast::Symbol::new(self.lexer.text(token)));
                        }
                        _ => {
                            self.lexer.back(token);
                            break;
                        }
                    }
                    if self.lex_peek()? == Tok::Comma {
                        self.lex_read()?;
                    } else {
                        break;
                    }
                }
                self.expect(Tok::RBrace)?;
                ast::BindingPattern::Object(ast::ObjectBindingPattern { props: props })
            }
            // Tok::LSquare => {
            //     // Array binding pattern.
            // }
            _ => return Err(self.parse_error(token, "binding element")),
        })
    }

    // 14.1 Function Definitions
    fn function(&mut self) -> ParseResult<ast::Function> {
        let name = match self.lex_peek()? {
            Tok::Ident => {
                let token = self.lex_read()?;
                Some(ast::Symbol::new(self.lexer.text(token)))
            }
            _ => None,
        };
        self.function_from_paren(name)
    }

    fn function_from_paren(&mut self, name: Option<Rc<ast::Symbol>>) -> ParseResult<ast::Function> {
        self.expect(Tok::LParen)?;
        let mut params: Vec<(ast::BindingPattern, Option<ExprNode>)> = Vec::new();
        while self.lex_peek()? != Tok::RParen {
            let binding = self.binding_pattern()?;
            let mut init: Option<ExprNode> = None;
            if self.lex_peek()? == Tok::Eq {
                self.lex_read()?;
                init = Some(self.expr_prec(3 /* assignment expr */)?);
            }
            params.push((binding, init));

            if self.lex_peek()? == Tok::Comma {
                self.lex_read()?;
            } else {
                break;
            }
        }
        self.expect(Tok::RParen)?;

        self.expect(Tok::LBrace)?;
        let mut body: Vec<Stmt> = Vec::new();
        while self.lex_peek()? != Tok::RBrace {
            body.push(self.stmt()?);
        }
        self.expect(Tok::RBrace)?;

        Ok(ast::Function {
            scope: ast::Scope::new(),
            name: name,
            params: params,
            body: body,
        })
    }

    fn function_call(&mut self, func: ExprNode) -> ParseResult<ExprNode> {
        let mut params: Vec<ExprNode> = Vec::new();
        loop {
            if self.lex_peek()? == Tok::RParen {
                break;
            }
            params.push(self.expr_prec(3)?);
            if self.lex_peek()? == Tok::Comma {
                self.lex_read()?;
                continue;
            }
            break;
        }
        let end = self.expect(Tok::RParen)?;
        Ok((
            Span::new(func.0.start, end),
            Expr::Call(Box::new(ast::Call {
                func: func,
                args: params,
            })),
        ))
    }

    fn expr_prec(&mut self, prec: usize) -> ParseResult<ExprNode> {
        // prec is precedence:
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Operator_Precedence

        // Parse a unary preceding op, or the primary expression itself.
        let token = self.lex_read()?;
        let mut expr = match token.tok {
            Tok::Not
            | Tok::BNot
            | Tok::Plus
            | Tok::Minus
            | Tok::PlusPlus
            | Tok::MinusMinus
            | Tok::Void
            | Tok::Delete if prec <= 16 =>
            {
                let expr = self.expr_prec(16)?;
                (
                    Span::new(token.span.start, expr.0.end),
                    Expr::Unary(ast::UnOp::from_tok(token.tok), Box::new(expr)),
                )
            }
            Tok::TypeOf if prec <= 16 => {
                let expr = self.expr_prec(16)?;
                (
                    Span::new(token.span.start, expr.0.end),
                    Expr::TypeOf(Box::new(expr)),
                )
            }
            Tok::New if prec <= 18 => {
                let expr = self.expr_prec(18)?;
                (
                    Span::new(token.span.start, expr.0.end),
                    Expr::New(Box::new(expr)),
                )
            }
            _ => {
                self.lexer.back(token);
                self.primary_expr()?
            }
        };

        // Parse any following unary/binary ops.
        loop {
            let token = self.lex_read()?;
            match token.tok {
                Tok::Comma if prec <= 0 => {
                    let rhs = self.expr_prec(0)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::Comma,
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::Eq if prec <= 3 => {
                    let rhs = self.expr_prec(3)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Assign(Box::new(expr), Box::new(rhs)),
                    );
                }
                Tok::PlusEq
                | Tok::MinusEq
                | Tok::StarStarEq
                | Tok::StarEq
                | Tok::DivEq
                | Tok::PercentEq
                | Tok::LTLTEq
                | Tok::GTGTEq
                | Tok::GTGTGTEq
                | Tok::AndEq
                | Tok::OrEq
                | Tok::CaratEq
                | Tok::OrEq if prec <= 3 =>
                {
                    let rhs = self.expr_prec(3)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::from_tok(token.tok),
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::Arrow if prec <= 3 => {
                    let start = expr.0.start;
                    let end = start; // TODO
                    let mut params: ast::ParameterList = Vec::new();
                    arrow_params_from_expr(expr, &mut params)?;
                    let body = if self.lex_peek()? == Tok::LBrace {
                        self.lex_read()?;
                        let mut body: Vec<Stmt> = Vec::new();
                        while self.lex_peek()? != Tok::RBrace {
                            body.push(self.stmt()?);
                        }
                        self.expect(Tok::RBrace)?;
                        ast::ArrowBody::Stmts(body)
                    } else {
                        ast::ArrowBody::Expr(self.expr_prec(3 /* assignment expr */)?)
                    };
                    expr = (
                        Span::new(start, end),
                        Expr::ArrowFunction(Box::new(ast::ArrowFunction {
                            params: params,
                            body: body,
                        })),
                    );
                }
                Tok::Question if prec <= 4 => {
                    let iftrue = self.expr_prec(3)?;
                    self.expect(Tok::Colon)?;
                    let iffalse = self.expr_prec(3)?;
                    expr = (
                        Span::new(expr.0.start, iffalse.0.end),
                        Expr::Ternary(Box::new(ast::Ternary {
                            condition: expr,
                            iftrue: iftrue,
                            iffalse: iffalse,
                        })),
                    );
                }
                Tok::OrOr if prec <= 5 => {
                    let rhs = self.expr_prec(5)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::OrOr,
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::AndAnd if prec <= 6 => {
                    let rhs = self.expr_prec(6)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::AndAnd,
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::BOr if prec <= 7 => {
                    let rhs = self.expr_prec(7)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::BOr,
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::Xor if prec <= 8 => {
                    let rhs = self.expr_prec(8)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::Xor,
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::BAnd if prec <= 9 => {
                    let rhs = self.expr_prec(9)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::BAnd,
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::EqEq | Tok::NEq | Tok::EqEqEq | Tok::NEqEq if prec <= 10 => {
                    let rhs = self.expr_prec(11)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::from_tok(token.tok),
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::LT | Tok::LTE | Tok::GT | Tok::GTE | Tok::In | Tok::InstanceOf
                    if prec <= 11 =>
                {
                    let rhs = self.expr_prec(11)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::from_tok(token.tok),
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::LTLT | Tok::GTGT | Tok::GTGTGT if prec <= 12 => {
                    let rhs = self.expr_prec(12)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::from_tok(token.tok),
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::Plus | Tok::Minus if prec <= 13 => {
                    let rhs = self.expr_prec(14)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::from_tok(token.tok),
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::Star | Tok::Div | Tok::Percent if prec <= 14 => {
                    let rhs = self.expr_prec(15)?;
                    expr = (
                        Span::new(expr.0.start, rhs.0.end),
                        Expr::Binary(Box::new(ast::Binary {
                            op: ast::BinOp::from_tok(token.tok),
                            lhs: expr,
                            rhs: rhs,
                        })),
                    );
                }
                Tok::PlusPlus if prec <= 17 => {
                    expr = (
                        Span::new(expr.0.start, token.span.end),
                        Expr::Unary(ast::UnOp::PostPlusPlus, Box::new(expr)),
                    )
                }
                Tok::MinusMinus if prec <= 17 => {
                    expr = (
                        Span::new(expr.0.start, token.span.end),
                        Expr::Unary(ast::UnOp::PostMinusMinus, Box::new(expr)),
                    )
                }
                Tok::Dot if prec <= 19 => {
                    let token = self.lex_read()?;
                    if token.tok != Tok::Ident && !token.tok.is_kw() {
                        return Err(self.parse_error(token, "member"));
                    }
                    let span = Span::new(expr.0.start, token.span.end);
                    let field = self.lexer.text(token);
                    expr = (span, Expr::Field(Box::new(expr), field));
                }
                Tok::LSquare if prec <= 19 => {
                    let index = self.expr()?;
                    let end = self.expect(Tok::RSquare)?;
                    expr = (
                        Span::new(index.0.start, end),
                        Expr::Index(Box::new(expr), Box::new(index)),
                    );
                }
                Tok::LParen if prec <= 19 => {
                    expr = self.function_call(expr)?;
                }
                _ => {
                    self.lexer.back(token);
                    return Ok(expr);
                }
            }
        }
    }

    pub fn expr(&mut self) -> ParseResult<ExprNode> {
        self.expr_prec(0)
    }

    fn bindings(&mut self) -> ParseResult<Vec<ast::VarDecl>> {
        let mut decls: Vec<ast::VarDecl> = Vec::new();
        loop {
            let pattern = self.binding_pattern()?;
            let init = if self.lex_peek()? == Tok::Eq {
                self.lex_read()?;
                Some(self.expr_prec(1)?)
            } else {
                None
            };
            decls.push(ast::VarDecl {
                pattern: pattern,
                init: init,
            });

            if self.lex_peek()? == Tok::Comma {
                self.lex_read()?;
                continue;
            } else {
                break;
            }
        }
        Ok(decls)
    }

    fn if_stmt(&mut self) -> ParseResult<ast::If> {
        self.expect(Tok::LParen)?;
        let cond = self.expr()?;
        self.expect(Tok::RParen)?;
        let iftrue = self.stmt()?;
        let else_ = if self.lex_peek()? == Tok::Else {
            self.lex_read()?;
            Some(try!(self.stmt()))
        } else {
            None
        };
        Ok(ast::If {
            cond: cond.1,
            iftrue: iftrue,
            else_: else_,
        })
    }

    fn while_stmt(&mut self) -> ParseResult<ast::While> {
        self.expect(Tok::LParen)?;
        let cond = self.expr()?;
        self.expect(Tok::RParen)?;
        let body = self.stmt()?;
        Ok(ast::While {
            cond: cond.1,
            body: body,
        })
    }

    fn do_while_stmt(&mut self) -> ParseResult<ast::While> {
        let body = self.stmt()?;
        self.expect(Tok::While)?;
        self.expect(Tok::LParen)?;
        let cond = self.expr()?;
        self.expect(Tok::RParen)?;
        self.expect_semi()?;
        Ok(ast::While {
            cond: cond.1,
            body: body,
        })
    }

    fn for_stmt(&mut self) -> ParseResult<Stmt> {
        // This is subtle because of the many possible forms of a 'for' statement.
        // See the test.
        // The important thing to open with is (unfortunately) a parse of the
        // body as an expr(), not anything more complex, because we need to parse
        // all of
        //   for (x = 3; ...
        //   for (x in y) ...
        //   for ([x] = 3; ...
        //   for ([x] in y; ...

        self.expect(Tok::LParen)?;

        let tok = self.lex_peek()?;
        let init = if tok == Tok::Semi {
            ast::ForInit::Empty
        } else {
            let decl_type = match tok {
                Tok::Var | Tok::Let | Tok::Const => {
                    self.lex_read()?;
                    Some(decl_type_from_tok(tok))
                }
                _ => None,
            };
            let expr = self.expr()?;
            // Check if it's a for-in-of loop.
            // Note: we want to match expr against an expr like
            //   a in b
            // and pull out 'a' and 'b', but it's a bit hard with borrowing.
            // So instead first check if it's a match, then match a second time
            // to consume it.
            if is_for_in_of_head(&expr.1) {
                let bin = match expr.1 {
                    ast::Expr::Binary(bin) => *bin,
                    _ => unreachable!(), // assured by is_for_in_of_head.
                };
                let ast::Binary { lhs, rhs, op } = bin;
                let loop_var = match lhs.1 {
                    ast::Expr::Ident(name) => ast::BindingPattern::Name(name),
                    _ => unimplemented!(),
                };
                self.expect(Tok::RParen)?;
                return Ok(Stmt::ForInOf(Box::new(ast::ForInOf {
                    decl_type: decl_type,
                    loop_var: loop_var,
                    expr: rhs,
                    body: self.stmt()?,
                })));
            }
            if let Some(decl_type) = decl_type {
                let mut decls: Vec<ast::VarDecl> = Vec::new();
                decls_from_expr(expr, &mut decls)?;
                ast::ForInit::Decls(ast::VarDecls {
                    typ: decl_type,
                    decls: decls,
                })
            } else {
                ast::ForInit::Expr(expr.1)
            }
        };
        self.expect(Tok::Semi)?;

        // for (a;b;c) loop.  Lexer is now pointed at 'b'.
        let cond = if self.lex_peek()? != Tok::Semi {
            Some(try!(self.expr()))
        } else {
            None
        };
        try!(self.expect(Tok::Semi));

        let iter = if self.lex_peek()? != Tok::RParen {
            Some(try!(self.expr()))
        } else {
            None
        };
        try!(self.expect(Tok::RParen));
        let body = try!(self.stmt());
        Ok(Stmt::For(Box::new(ast::For {
            init: init,
            cond: cond.map(|c| c.1),
            iter: iter.map(|i| i.1),
            body: body,
        })))
    }

    fn switch(&mut self) -> ParseResult<ast::Switch> {
        try!(self.expect(Tok::LParen));
        let expr = try!(self.expr());
        try!(self.expect(Tok::RParen));
        try!(self.expect(Tok::LBrace));
        let mut cases: Vec<ast::Case> = Vec::new();
        loop {
            cases.push(match self.lex_peek()? {
                Tok::Case => {
                    self.lex_read()?;
                    let expr = self.expr()?;
                    self.expect(Tok::Colon)?;
                    let stmts = self.stmts()?;
                    ast::Case {
                        expr: Some(expr.1),
                        stmts: stmts,
                    }
                }
                Tok::Default => {
                    self.lex_read()?;
                    try!(self.expect(Tok::Colon));
                    let stmts = try!(self.stmts());
                    ast::Case {
                        expr: None,
                        stmts: stmts,
                    }
                }
                _ => {
                    break;
                }
            });
        }
        self.expect(Tok::RBrace)?;
        Ok(ast::Switch {
            expr: expr.1,
            cases: cases,
        })
    }

    fn try(&mut self) -> ParseResult<ast::Try> {
        let block = self.block()?;

        let catch = if self.lex_peek()? == Tok::Catch {
            self.lex_read()?;
            self.expect(Tok::LParen)?;
            let catch_expr = self.binding_pattern()?;
            self.expect(Tok::RParen)?;
            let catch_block = self.block()?;
            Some((catch_expr, catch_block))
        } else {
            None
        };

        let finally = if self.lex_peek()? == Tok::Finally {
            self.lex_read()?;
            let fin_block = self.block()?;
            Some(fin_block)
        } else {
            None
        };

        Ok(ast::Try {
            block: block,
            catch: catch,
            finally: finally,
        })
    }

    // Attempt to read a semicolon, returning true if succeeded.
    fn asi_semi(&mut self) -> ParseResult<bool> {
        let token = self.lex_read()?;
        Ok(match token.tok {
            Tok::Semi => true,
            // Attempt ASI if not a semi.
            _ if token.saw_newline => {
                self.lexer.back(token);
                true
            }
            Tok::RBrace | Tok::EOF => {
                self.lexer.back(token);
                true
            }
            _ => {
                self.lexer.back(token);
                false
            }
        })
    }

    fn expect_semi(&mut self) -> ParseResult<()> {
        if !self.asi_semi()? {
            let token = self.lex_read()?;
            return Err(self.parse_error(token, "semicolon"));
        }
        Ok(())
    }

    // 13 ECMAScript Language: Statements and Declarations
    pub fn stmt(&mut self) -> ParseResult<Stmt> {
        let token = self.lex_read()?;
        let stmt = match token.tok {
            // Declaration
            Tok::Class => Stmt::Class(Box::new(self.class()?)),
            Tok::Function => Stmt::Function(Box::new(self.function()?)),
            // Statement
            Tok::LBrace => {
                let body = try!(self.stmts());
                try!(self.expect(Tok::RBrace));
                Stmt::Block(body)
            }
            Tok::Var | Tok::Let | Tok::Const => {
                let typ = decl_type_from_tok(token.tok);
                let decls = self.bindings()?;
                self.expect_semi()?;
                Stmt::Var(Box::new(ast::VarDecls {
                    typ: typ,
                    decls: decls,
                }))
            }
            Tok::Semi => Stmt::Empty,
            Tok::If => Stmt::If(Box::new(try!(self.if_stmt()))),
            Tok::While => Stmt::While(Box::new(try!(self.while_stmt()))),
            Tok::Do => Stmt::DoWhile(Box::new(try!(self.do_while_stmt()))),
            Tok::For => try!(self.for_stmt()),
            Tok::Switch => Stmt::Switch(Box::new(try!(self.switch()))),
            Tok::Break | Tok::Continue => {
                let target = if self.asi_semi()? {
                    None
                } else {
                    let token = self.lex_read()?;
                    if token.tok != Tok::Ident {
                        return Err(self.parse_error(token, "label"));
                    }
                    let label = self.lexer.text(token);
                    self.expect_semi()?;
                    Some(label)
                };
                if token.tok == Tok::Break {
                    Stmt::Break(target)
                } else {
                    Stmt::Continue(target)
                }
            }
            Tok::Return => {
                if self.asi_semi()? {
                    Stmt::Return(None)
                } else {
                    let expr = self.expr()?;
                    self.expect_semi()?;
                    Stmt::Return(Some(Box::new(expr.1)))
                }
            }
            Tok::Throw => Stmt::Throw(Box::new(self.expr()?.1)),
            Tok::Try => Stmt::Try(Box::new(try!(self.try()))),
            t => {
                if t == Tok::Ident && self.lex_peek()? == Tok::Colon {
                    // Note: we have to read two tokens(!) to spot a label statement.
                    let label = self.lexer.text(token);
                    self.lex_read()?;
                    let stmt = try!(self.stmt());
                    return Ok(Stmt::Label(Box::new(ast::Label {
                        label: label,
                        stmt: stmt,
                    })));
                }
                self.lexer.back(token);
                let expr = self.expr()?;
                self.expect_semi()?;
                Stmt::Expr(expr)
            }
        };
        Ok(stmt)
    }

    pub fn stmts(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut body: Vec<Stmt> = Vec::new();
        loop {
            match self.lex_peek()? {
                // TODO: why 'case' here? because otherwise we read error on cases in switch.
                Tok::RBrace | Tok::EOF | Tok::Case | Tok::Default => break,
                _ => {
                    body.push(try!(self.stmt()));
                }
            }
        }
        Ok(body)
    }

    // In some cases, the syntax only allows a block but for AST simplicity
    // we represent it as a single Stmt::Block.
    fn block(&mut self) -> ParseResult<Stmt> {
        try!(self.expect(Tok::LBrace));
        let block = try!(self.stmts());
        try!(self.expect(Tok::RBrace));
        Ok(Stmt::Block(block))
    }

    pub fn module(&mut self) -> ParseResult<ast::Module> {
        Ok(ast::Module {
            scope: ast::Scope::new(),
            stmts: self.stmts()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_expr(input: &str) -> Expr {
        println!("parse_expr: {:?}", input);
        let mut p = Parser::new(input.as_bytes());
        let (_, expr) = match p.expr() {
            Err(err) => {
                err.print(&p.lexer);
                panic!("err");
            }
            Ok(expr) => expr,
        };
        println!("result: {:?}", expr);
        p.expect(Tok::EOF).unwrap();
        expr
    }

    fn parse(input: &str) -> Vec<Stmt> {
        println!("parse: {:?}", input);
        let mut p = Parser::new(input.as_bytes());
        let stmts = match p.stmts() {
            Err(err) => {
                err.print(&p.lexer);
                panic!("err");
            }
            Ok(stmt) => stmt,
        };
        println!("result: {:?}", stmts);
        p.expect(Tok::EOF).unwrap();
        stmts
    }

    #[test]
    fn expr() {
        parse_expr("abc");
    }

    #[test]
    fn assoc() {
        let expr = parse_expr("a + b + c");
        match expr {
            Expr::Binary(ref bin) => match bin.lhs {
                (_, Expr::Binary(_)) => {}
                _ => panic!("fail"),
            },
            _ => panic!("fail"),
        }
    }

    #[test]
    fn binop() {
        parse_expr("a * b + c * d + e * f * g + h");

        parse_expr("i + j * k + l");

        parse_expr("(i + j) * k + l");
    }

    #[test]
    fn field() {
        parse_expr("a.b.c");
        //parse_expr("a.3");
    }

    #[test]
    fn unary() {
        parse_expr("a + typeof typeof a.b + a");
    }

    #[test]
    fn umd() {
        parse_expr(
            "function (global, factory) {
	typeof exports === 'object' && typeof module !== 'undefined' ? module.exports = factory() :
	typeof define === 'function' && define.amd ? define(factory) :
    (global.x = factory());
}",
        );
    }

    #[test]
    fn asi() {
        parse(
            "{ a
b } c",
        );
    }

    #[test]
    fn asi_case() {
        parse(
            "
      switch (c) {
        case 1: a; break // c1
        case 2: b; break // c2
      }",
        );
    }

    #[test]
    fn asi_return() {
        parse(
            "return
return 3
{ return }
{ return 3 }
return",
        );
    }

    #[test]
    fn asi_comment() {
        parse(
            "var foo = function() {}
/** comment */ x;",
        );
    }

    #[test]
    fn asi_comment_2() {
        parse(
            "var foo = function() {}  // x
x;",
        );
    }

    #[test]
    fn for_variants() {
        parse("for (;;);");
        parse("for (var x = 3; a; b);");
        parse("for (var x = 3, y = 4; a; b);");
        parse("for (x = 3; a; b);");
        parse("for (x = 3, y = 4; a; b);");
        parse("for (var x in a);");
        parse("for (const x in a);");
        parse("for (x in a);");
    }

    #[test]
    fn kws() {
        parse("foo.extends = bar.case;");
    }

    #[test]
    fn label() {
        parse("foo: bar;");
    }

    #[test]
    fn object() {
        parse("({a: b, 'a': b, a, a() {}, 0: 0, a});");
    }

    #[test]
    fn trailing_comma() {
        parse("f({a,}, [b,], );");
    }

    mod es6 {
        use super::*;

        #[test]
        fn class() {
            parse(
                "class C extends B {
  f() { super.x; }
  ;
  f2() {}
}",
            );
        }

        #[test]
        fn class_expr() {
            parse("let x = class {}");
            parse("let x = class A {}");
            parse("let x = class extends B {}");
        }

        #[test]
        fn function_binding() {
            parse("function f({a}) {}");
            parse("function f({a,b}) {}");
            parse("function f({a,b,}) {}");
            parse("function f({a} = {}) {}");
        }

        #[test]
        fn let_const() {
            parse("let x = 3, y;");
            parse("const x, y = 4;");
        }

        #[test]
        fn let_binding() {
            parse("const {x} = a;");
        }

        #[test]
        fn arrow() {
            parse("x => 3");
            parse("() => 3");
            parse("(a, b, c) => 3");
        }
    }
}
