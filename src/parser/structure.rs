use std::rc::Rc;
use crate::parser::grammar::{GuardedToken, Token};

#[derive(Debug, Clone, PartialEq)]
pub enum Delimiter {
    Parentheses,
    Braces,
    Brackets
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenGroup {
    pub delimiter: Delimiter,
    pub children: Rc<[TokenNode]>,
    pub terminated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenNode {
    Token(GuardedToken),
    Group(TokenGroup)
}

impl TokenNode {
    pub fn try_get_token(&self) -> Option<GuardedToken> {
        if let TokenNode::Token(token) = self {
            Some(token.clone())
        } else {
            None
        }
    }
    
    pub fn try_get_group(&self) -> Option<TokenGroup> {
        if let TokenNode::Group(group) = self {
            Some(group.clone())
        } else {
            None
        }
    }
}

pub fn collect_token_nodes(nodes: &[GuardedToken]) -> Vec<TokenNode> {
    let mut index = 0usize;
    let (_, nodes) = collect_until(nodes, &mut index, None);
    nodes
}

fn collect_until(nodes: &[GuardedToken], index: &mut usize, delimiter: Option<Token>) -> (bool, Vec<TokenNode>) {
    let mut result = Vec::new();
    while let Some(guarded) = nodes.get(*index) {
        *index += 1;

        if delimiter.as_ref().is_some_and(|d| *d == guarded.token) {
            return (true, result);
        }

        match guarded.token {
            Token::LBrace => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RBrace));
                result.push(TokenNode::Group(TokenGroup {
                    delimiter: Delimiter::Braces,
                    children: Rc::from(children),
                    terminated,
                }));
            }
            Token::LBracket => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RBracket));
                result.push(TokenNode::Group(TokenGroup {
                    delimiter: Delimiter::Brackets,
                    children: Rc::from(children),
                    terminated,
                }));
            }
            Token::LParen => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RParen));
                result.push(TokenNode::Group( TokenGroup{
                    delimiter: Delimiter::Parentheses,
                    children: Rc::from(children),
                    terminated,
                }));
            }
            _ => {
                result.push(TokenNode::Token(guarded.clone()));
            }
        }
    }

    (false, result)
}