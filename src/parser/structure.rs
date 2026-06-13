use crate::parser::grammar::{GuardedToken, Token};
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Delimiter {
    Parentheses,
    Braces,
    Brackets,
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
    Group(TokenGroup),
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

    pub fn is_attribute(&self) -> bool {
        self.try_get_group().is_some_and(|group| {
            group.delimiter == Delimiter::Brackets
                && group.children.len() == 1
                && group.children[0]
                    .try_get_group()
                    .is_some_and(|group| group.delimiter == Delimiter::Brackets)
        })
    }
}

impl Display for TokenNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenNode::Token(token) => write!(f, "{}", token),
            TokenNode::Group(group) =>
            match group.delimiter {
                Delimiter::Parentheses => write!(f, "(...)"),
                Delimiter::Braces => write!(f, "{{...}}"),
                Delimiter::Brackets => write!(f, "[...]"),
            },
        }
    }
}

pub fn collect_token_nodes(nodes: &[GuardedToken]) -> Vec<TokenNode> {
    let mut index = 0usize;
    let (_, nodes) = collect_until(nodes, &mut index, None);
    nodes
}

fn collect_until(
    nodes: &[GuardedToken],
    index: &mut usize,
    delimiter: Option<Token>,
) -> (bool, Vec<TokenNode>) {
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
                let node = TokenNode::Group(TokenGroup {
                    delimiter: Delimiter::Brackets,
                    children: Rc::from(children),
                    terminated,
                });
                if node.is_attribute()
                {
                    continue;
                }
                result.push(node);
            }
            Token::LParen => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RParen));
                result.push(TokenNode::Group(TokenGroup {
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
