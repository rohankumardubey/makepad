#![allow(unused_variables)]
#![allow(unused_imports)]

use makepad_live_derive::*;
use crate::token::Token;
use crate::token::TokenWithSpan;
use std::iter::Cloned;
use std::slice::Iter;
use crate::span::{LiveFileId, Span};
use crate::liveerror::LiveError;
use crate::id::Id;
use crate::lex::LexResult;
use crate::livenode::{LiveDocument, LiveNode, LiveValue};

pub struct LiveParser<'a> {
    //pub token_clone: Vec<TokenWithSpan>,
    pub lex_result: &'a LexResult,
    pub token_index: usize,
    pub live_file_id: u32,
    pub tokens_with_span: Cloned<Iter<'a, TokenWithSpan >>,
    pub token_with_span: TokenWithSpan,
    pub end: usize,
}

impl<'a> LiveParser<'a> {
    pub fn new(lex_result: &'a LexResult) -> Self {
        let mut tokens_with_span = lex_result.tokens.iter().cloned();
        let token_with_span = tokens_with_span.next().unwrap();
        LiveParser {
            lex_result,
            live_file_id: 0,
            //token_clone: Vec::new(),
            tokens_with_span,
            token_with_span,
            token_index: 0,
            end: 0,
        }
    }
}

impl<'a> LiveParser<'a> {
    
    /*fn clear_token_clone(&mut self) {
        self.token_clone.truncate(0);
    }*/
    
    /*fn get_token_clone(&mut self) -> Vec<TokenWithSpan> {
        let mut new_token_storage = Vec::new();
        std::mem::swap(&mut new_token_storage, &mut self.token_clone);
        new_token_storage.push(TokenWithSpan{
            token:Token::Eof,
            span:self.token_with_span.span
        });
        return new_token_storage;
    }*/
    
    fn peek_span(&self) -> Span {
        self.token_with_span.span
    }
    
    fn peek_token(&self) -> Token {
        self.token_with_span.token
    }
    
    fn eat_token(&mut self) -> Token {
        let token = self.peek_token();
        self.skip_token();
        token
    }
    
    fn skip_token(&mut self) {
        self.end = self.token_with_span.span.end as usize;
        //self.token_clone.push(self.token_with_span);
        self.token_with_span = self.tokens_with_span.next().unwrap();
        self.token_index += 1;
    }
    
    fn error(&mut self, message: String) -> LiveError {
        LiveError {
            span: Span {
                live_file_id: self.token_with_span.span.live_file_id,
                start: self.token_with_span.span.start,
                end: self.token_with_span.span.end,
            },
            message,
        }
    }
    
    fn expect_ident(&mut self) -> Result<Id, LiveError> {
        match self.peek_token() {
            Token::Ident(ident) => {
                self.skip_token();
                Ok(ident)
            }
            token => Err(self.error(format!("expected ident, unexpected token `{}`", token))),
        }
    }
    
    fn end(&self) -> usize {
        self.end
    }
    
    fn token_end(&self) -> usize {
        self.token_with_span.span.end as usize
    }
    
    fn accept_ident(&mut self, token: Token) -> Option<Id> {
        if let Token::Ident(id) = self.peek_token() {
            self.skip_token();
            Some(id)
        }
        else {
            None
        }
    }
    
    fn accept_token(&mut self, token: Token) -> bool {
        if self.peek_token() != token {
            return false;
        }
        self.skip_token();
        true
    }
    
    fn expect_token(&mut self, expected: Token) -> Result<(), LiveError> {
        let actual = self.peek_token();
        if actual != expected {
            return Err(self.error(format!("expected {} unexpected token `{}`", expected, actual)));
        }
        self.skip_token();
        Ok(())
    }
    
    fn begin_span(&self) -> SpanTracker {
        SpanTracker {
            live_file_id: self.token_with_span.span.live_file_id,
            start: self.token_with_span.span.start as usize,
        }
    }
    
    fn expect_class_id(&mut self, ld: &mut LiveDocument) -> Result<Id, LiveError> {
        let base = self.expect_ident() ?;
        
        if self.peek_token() == token_punct!(::) {
            self.skip_token();
            // start a multi_id
            let multi_index = ld.multi_ids.len();
            ld.multi_ids.push(base);
            loop {
                match self.peek_token() {
                    Token::Ident(id) => {
                        self.skip_token();
                        ld.multi_ids.push(id);
                        if !self.accept_token(token_punct!(::)) {
                            break;
                        }
                    },
                    other => {
                        return Err(self.error(format!("Unexpected token after :: {}", other)));
                    }
                }
            };
            let id = Id::multi(multi_index as u32, (ld.multi_ids.len() - multi_index) as u32);
            Ok(id)
        }
        else {
            Ok(base)
        }
    }
    
    fn expect_prop_id(&mut self, ld: &mut LiveDocument) -> Result<Id, LiveError> {
        let base = self.expect_ident() ?;
        if self.peek_token() == token_punct!(.) {
            self.skip_token();
            // start a multi_id
            let multi_index = ld.multi_ids.len();
            ld.multi_ids.push(base);
            loop {
                match self.peek_token() {
                    Token::Ident(id) => {
                        self.skip_token();
                        ld.multi_ids.push(id);
                        if !self.accept_token(token_punct!(.)) {
                            break;
                        }
                    },
                    other => {
                        return Err(self.error(format!("Unexpected token after . {}", other)));
                    }
                }
            };
            let id = Id::multi(multi_index as u32, (ld.multi_ids.len() - multi_index) as u32);
            Ok(id)
        }
        else {
            Ok(base)
        }
    }
    
    fn expect_object(&mut self, level: usize, ld: &mut LiveDocument) -> Result<(u32,u32), LiveError>{
        let node_start = ld.level_len(level);
        while self.peek_token() != Token::Eof {
            if self.peek_token() == Token::CloseBrace {
                self.skip_token();
                let node_end = ld.level_len(level);
                return Ok((node_start as u32, (node_end - node_start) as u32))
            }
            self.expect_live_value(Id::empty(), level, ld)?;
            self.expect_token(token_punct!(:))?;
            self.expect_live_value(Id::empty(), level, ld)?;
            if !self.accept_token(token_punct!(,)) {
                self.accept_token(token_punct!(;));
            }
        }
        return Err(self.error(format!("Eof in object body")))
    }
    
    fn expect_array(&mut self, level: usize, ld: &mut LiveDocument) -> Result<(u32,u32), LiveError>{
        let node_start = ld.level_len(level);
        while self.peek_token() != Token::Eof {
            if self.peek_token() == Token::CloseBracket {
                self.skip_token();
                let node_end = ld.level_len(level);
                return Ok((node_start as u32, (node_end - node_start) as u32))
            }
            self.expect_live_value(Id::empty(), level, ld)?;
            self.accept_token(token_punct!(,));
        }
        return Err(self.error(format!("Eof in object body")))
    }        

    fn expect_live_value(&mut self, prop_id: Id, level: usize, ld: &mut LiveDocument) -> Result<(), LiveError> {
        
        // now we can have an array or a class instance
        match self.peek_token() {
            Token::OpenBrace => { // key/value map
                self.skip_token();
                let (node_start, node_count) = self.expect_object(level + 1, ld) ?;
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::Object {node_start, node_count}
                });
            },
            Token::OpenBracket => { // array
                self.skip_token();
                let (node_start, node_count) = self.expect_array(level + 1, ld) ?;
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::Array {node_start, node_count}
                });
            },
            Token::Bool(val) => {
                self.skip_token();
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::Bool(val)
                });
            },
            Token::Int(val) => {
                self.skip_token();
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::Int(val)
                });
            },
            Token::Float(val) => {
                self.skip_token();
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::Float(val)
                });
            },
            Token::Color(val) => {
                self.skip_token();
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::Color(val)
                });
            },
            Token::String{index, len}=>{
                self.skip_token();
                ld.push_node(level, LiveNode {
                    id: prop_id,
                    value: LiveValue::String{string_index:index, string_len:len}
                });
            },
            token_ident!(vec2) => {
            },
            token_ident!(vec3) => {
            },
            Token::Ident(id) => { // we're gonna parse a class.
                // we also support vec2/vec3 values directly.
                let class = self.expect_class_id(ld) ?;
                if self.accept_token(Token::OpenBrace) {
                    let (node_start, node_count) = self.expect_live_class(level + 1, ld) ?;
                    ld.push_node(level, LiveNode {
                        id: prop_id,
                        value: LiveValue::Class {class, node_start, node_count}
                    });
                }
                else {
                    ld.push_node(level, LiveNode {
                        id: prop_id,
                        value: LiveValue::Id(class)
                    });
                }
            },
            other => return Err(self.error(format!("Unexpected token {} in property value", other)))
        }
        Ok(())
    }
    
    fn scan_to_token(&mut self, scan_token: Token) -> Result<(u32, u32), LiveError> {
        // ok we are going to scan to token, keeping in mind our levels.
        let mut stack_depth = 0;
        let token_start = self.token_index;
        
        while self.peek_token() != Token::Eof {
            match self.peek_token() {
                Token::OpenBrace | Token::OpenParen | Token::OpenBracket => {
                    stack_depth += 1;
                }
                Token::CloseBrace | Token::CloseParen | Token::CloseBracket => {
                    if stack_depth == 0 {
                        return Err(self.error(format!("Found closing )}}] whilst scanning for {}", scan_token)));
                    }
                    stack_depth -= 1;
                }
                _ => ()
            }
            if stack_depth == 0 && self.peek_token() == scan_token {
                self.skip_token();
                let token_count = (self.token_index - token_start) as u32;
                return Ok((token_start as u32, token_count))
            }
            self.skip_token();
        }
        return Err(self.error(format!("Could not find ending token {} whilst scanning", scan_token)));
    }
    
    fn expect_live_class(&mut self, level: usize, ld: &mut LiveDocument) -> Result<(u32, u16), LiveError> {
        let node_start = ld.level_len(level);
        while self.peek_token() != Token::Eof {
            match self.peek_token() {
                token_ident!(use) => {
                    self.skip_token();
                    if level != 0 {
                        return Err(self.error(format!("use statement only allowed at the root level")));
                    }
                    let crate_name = self.expect_ident() ?;
                    self.expect_token(token_punct!(::)) ?;
                    // now we either get a {  or a single ident
                    if self.accept_token(Token::OpenBrace) {
                        loop {
                            match self.eat_token() {
                                Token::Eof | Token::CloseBrace => {
                                    self.accept_token(token_punct!(;));
                                    break
                                }
                                Token::Ident(crate_import) => {
                                    ld.add_use_import(crate_name, crate_import);
                                }
                                token_punct!(,) => {
                                }
                                other => return Err(self.error(format!("Unexpected token {} in use list", other)))
                            }
                        }
                    }
                    else {
                        ld.add_use_import(crate_name, self.expect_ident() ?);
                    }
                },
                token_ident!(const) => {
                    self.skip_token();
                    let prop_id = self.expect_ident() ?;
                    
                    let (token_start, token_count) = self.scan_to_token(token_punct!(;)) ?;
                    
                    ld.push_node(level, LiveNode {
                        id: prop_id,
                        value: LiveValue::Const {live_file_id: self.live_file_id, token_start, token_count}
                    });
                },
                token_ident!(fn) => {
                    self.skip_token();
                    let prop_id = self.expect_ident() ?;
                    let (token_start, token_count) = self.scan_to_token(Token::CloseBrace) ?;
                    
                    ld.push_node(level, LiveNode {
                        id: prop_id,
                        value: LiveValue::Fn {live_file_id: self.live_file_id, token_start, token_count}
                    });
                },
                Token::CloseBrace => {
                    self.skip_token();
                    let node_end = ld.level_len(level);
                    return Ok((node_start as u32, (node_end - node_start) as u16))
                }
                Token::Ident(prop) => {
                    let prop_id = self.expect_prop_id(ld) ?;
                    self.expect_token(token_punct!(:)) ?;
                    // ok now we get a value to parse
                    self.expect_live_value(prop_id, level, ld) ?;
                    if !self.accept_token(token_punct!(,)) {
                        self.accept_token(token_punct!(;));
                    }
                },
                other => return Err(self.error(format!("Unexpected token {} in class body", other)))
            }
        }
        if level == 0{
            let node_end = ld.level_len(level);
            return Ok((node_start as u32, (node_end - node_start) as u16))
        }
        return Err(self.error(format!("Eof in class body")))
    }
    
    pub fn parse_live_document(&mut self) -> Result<LiveDocument, LiveError> {
        let mut ld = LiveDocument::new();
        self.expect_live_class(0, &mut ld) ?;
        // we should s
        Ok(ld)
    }
}


pub struct SpanTracker {
    pub live_file_id: LiveFileId,
    pub start: usize,
}

impl SpanTracker {
    pub fn end<F, R>(&self, parser: &mut LiveParser, f: F) -> R
    where
    F: FnOnce(Span) -> R,
    {
        f(Span {
            live_file_id: self.live_file_id,
            start: self.start as u32,
            end: parser.end() as u32,
        })
    }
    
    pub fn error(&self, parser: &mut LiveParser, message: String) -> LiveError {
        LiveError {
            span: Span {
                live_file_id: self.live_file_id,
                start: self.start as u32,
                end: parser.token_end() as u32,
            },
            message,
        }
    }
}
