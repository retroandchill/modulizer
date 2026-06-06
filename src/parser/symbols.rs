use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fmt::Formatter;
use crate::config::Config;
use crate::parser::grammar::Token;

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub enum Symbol {
    Namespace(Namespace),
}