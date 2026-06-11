use crate::parser::grammar::{GuardedToken, Token};

#[derive(Debug, PartialEq)]
pub enum Delimiter {
    Parentheses,
    Braces,
    Brackets
}

pub struct TokenGroup<'tok> {
    pub delimiter: Delimiter,
    pub children: Vec<TokenNode<'tok>>,
    pub terminated: bool,
}

pub enum TokenNode<'tok> {
    Token(GuardedToken<'tok>),
    Group(TokenGroup<'tok>)
}

pub fn collect_token_nodes<'tok>(nodes: &[GuardedToken<'tok>]) -> Vec<TokenNode<'tok>> {
    let mut index = 0usize;
    let (_, nodes) = collect_until(nodes, &mut index, None);
    nodes
}

fn collect_until<'tok>(nodes: &[GuardedToken<'tok>], index: &mut usize, delimiter: Option<Token>) -> (bool, Vec<TokenNode<'tok>>) {
    let mut result = Vec::new();
    while let Some(guarded) = nodes.get(*index) {
        *index += 1;

        if delimiter.as_ref().is_some_and(|d| *d == *guarded.token) {
            return (true, result);
        }

        match guarded.token {
            Token::LBrace => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RBrace));
                result.push(TokenNode::Group(TokenGroup {
                    delimiter: Delimiter::Braces,
                    children,
                    terminated,
                }));
            }
            Token::LBracket => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RBracket));
                result.push(TokenNode::Group(TokenGroup {
                    delimiter: Delimiter::Brackets,
                    children,
                    terminated,
                }));
            }
            Token::LParen => {
                let (terminated, children) = collect_until(nodes, index, Some(Token::RParen));
                result.push(TokenNode::Group( TokenGroup{
                    delimiter: Delimiter::Parentheses,
                    children,
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