use pest::Parser;
use pest::RuleType;
use pest::prec_climber::{PrecClimber, Operator, Assoc};
use pest::iterators::Pair;
use crate::env::Env;
use crate::symbols::Symbol;
use crate::error::Error;
use crate::distr::KeyType;


#[derive(Parser)]
#[grammar = "grammar.pest"]
struct Grammar;

pub fn parse_line(src: &str, env: &Env) -> Result<Symbol, Error> {
    let climber = PrecClimber::new(vec![
        Operator::new(Rule::add, Assoc::Left) | Operator::new(Rule::sub, Assoc::Right),
        Operator::new(Rule::apply, Assoc::Left)
    ]);

    let mut pairs: _ = Grammar::parse(Rule::line, src).unwrap();

    let primary = |pair| { pair };
    let infix = |lhs, op, rhs| {
        println!("infix: {} {} {}", lhs, op, rhs);
        lhs
    };
    let t = climber.climb(pairs, primary, infix);
    Ok(().into())
}

fn make_symbol(pair: Pair<Rule>) -> Result<Symbol, Error> {
    panic!()
}