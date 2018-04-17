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

// GENERATED by genscan.rs
use std::fmt;
use lex::Tok;
#[derive(Debug, PartialEq)]
pub enum UnOp {
    Not,
    BNot,
    PlusPlus,
    MinusMinus,
    Minus,
    Plus,
    PostMinusMinus,
    PostPlusPlus,
    Await,
    Delete,
    TypeOf,
    Void,
}
impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            &UnOp::Not => "!",
            &UnOp::BNot => "~",
            &UnOp::PlusPlus => "++",
            &UnOp::MinusMinus => "--",
            &UnOp::Minus => "-",
            &UnOp::Plus => "+",
            &UnOp::PostMinusMinus => "--",
            &UnOp::PostPlusPlus => "++",
            &UnOp::Await => "await",
            &UnOp::Delete => "delete",
            &UnOp::TypeOf => "typeof",
            &UnOp::Void => "void",
        })
    }
}
impl UnOp {
    pub fn from_tok(t: Tok) -> UnOp {
        match t {
            Tok::Not => UnOp::Not,
            Tok::BNot => UnOp::BNot,
            Tok::PlusPlus => UnOp::PlusPlus,
            Tok::MinusMinus => UnOp::MinusMinus,
            Tok::Minus => UnOp::Minus,
            Tok::Plus => UnOp::Plus,
            Tok::Await => UnOp::Await,
            Tok::Delete => UnOp::Delete,
            Tok::TypeOf => UnOp::TypeOf,
            Tok::Void => UnOp::Void,
            _ => panic!("non-UnOp {:?}", t),
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum BinOp {
    Eq,
    LT,
    GT,
    LTE,
    GTE,
    EqEq,
    NEq,
    EqEqEq,
    NEqEq,
    Plus,
    Minus,
    Star,
    Percent,
    StarStar,
    LTLT,
    GTGT,
    GTGTGT,
    BAnd,
    BOr,
    Xor,
    AndAnd,
    OrOr,
    PlusEq,
    MinusEq,
    StarEq,
    PercentEq,
    StarStarEq,
    LTLTEq,
    GTGTEq,
    GTGTGTEq,
    AndEq,
    OrEq,
    CaratEq,
    Div,
    DivEq,
    Comma,
    In,
    InstanceOf,
}
impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            &BinOp::Eq => "=",
            &BinOp::LT => "<",
            &BinOp::GT => ">",
            &BinOp::LTE => "<=",
            &BinOp::GTE => ">=",
            &BinOp::EqEq => "==",
            &BinOp::NEq => "!=",
            &BinOp::EqEqEq => "===",
            &BinOp::NEqEq => "!==",
            &BinOp::Plus => "+",
            &BinOp::Minus => "-",
            &BinOp::Star => "*",
            &BinOp::Percent => "%",
            &BinOp::StarStar => "**",
            &BinOp::LTLT => "<<",
            &BinOp::GTGT => ">>",
            &BinOp::GTGTGT => ">>>",
            &BinOp::BAnd => "&",
            &BinOp::BOr => "|",
            &BinOp::Xor => "^",
            &BinOp::AndAnd => "&&",
            &BinOp::OrOr => "||",
            &BinOp::PlusEq => "+=",
            &BinOp::MinusEq => "-=",
            &BinOp::StarEq => "*=",
            &BinOp::PercentEq => "%=",
            &BinOp::StarStarEq => "**=",
            &BinOp::LTLTEq => "<<=",
            &BinOp::GTGTEq => ">>=",
            &BinOp::GTGTGTEq => ">>>=",
            &BinOp::AndEq => "&=",
            &BinOp::OrEq => "|=",
            &BinOp::CaratEq => "^=",
            &BinOp::Div => "/",
            &BinOp::DivEq => "/=",
            &BinOp::Comma => ",",
            &BinOp::In => "in",
            &BinOp::InstanceOf => "instanceof",
        })
    }
}
impl BinOp {
    pub fn from_tok(t: Tok) -> BinOp {
        match t {
            Tok::Eq => BinOp::Eq,
            Tok::LT => BinOp::LT,
            Tok::GT => BinOp::GT,
            Tok::LTE => BinOp::LTE,
            Tok::GTE => BinOp::GTE,
            Tok::EqEq => BinOp::EqEq,
            Tok::NEq => BinOp::NEq,
            Tok::EqEqEq => BinOp::EqEqEq,
            Tok::NEqEq => BinOp::NEqEq,
            Tok::Plus => BinOp::Plus,
            Tok::Minus => BinOp::Minus,
            Tok::Star => BinOp::Star,
            Tok::Percent => BinOp::Percent,
            Tok::StarStar => BinOp::StarStar,
            Tok::LTLT => BinOp::LTLT,
            Tok::GTGT => BinOp::GTGT,
            Tok::GTGTGT => BinOp::GTGTGT,
            Tok::BAnd => BinOp::BAnd,
            Tok::BOr => BinOp::BOr,
            Tok::Xor => BinOp::Xor,
            Tok::AndAnd => BinOp::AndAnd,
            Tok::OrOr => BinOp::OrOr,
            Tok::PlusEq => BinOp::PlusEq,
            Tok::MinusEq => BinOp::MinusEq,
            Tok::StarEq => BinOp::StarEq,
            Tok::PercentEq => BinOp::PercentEq,
            Tok::StarStarEq => BinOp::StarStarEq,
            Tok::LTLTEq => BinOp::LTLTEq,
            Tok::GTGTEq => BinOp::GTGTEq,
            Tok::GTGTGTEq => BinOp::GTGTGTEq,
            Tok::AndEq => BinOp::AndEq,
            Tok::OrEq => BinOp::OrEq,
            Tok::CaratEq => BinOp::CaratEq,
            Tok::Div => BinOp::Div,
            Tok::DivEq => BinOp::DivEq,
            Tok::Comma => BinOp::Comma,
            Tok::In => BinOp::In,
            Tok::InstanceOf => BinOp::InstanceOf,
            _ => panic!("non-BinOp {:?}", t),
        }
    }
}
