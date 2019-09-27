use pest::Parser;
use pest::iterators::{Pair, Pairs};
use crate::env::Env;
use crate::symbols::Symbol;
use crate::error::Error;
use crate::distr::{KeyType, Distr, ProbType};
use pest::prec_climber::{PrecClimber, Assoc, Operator};


#[derive(Parser)]
#[grammar = "grammar.pest"]
struct Grammar;

pub fn parse_line(src: &str, _env: &Env) -> Result<Symbol, Error> {
    match Grammar::parse(Rule::line, src) {
        Ok(pairs) => Ok(parse_expr(pairs)),
        Err(pest_err) => Err(fail!("{}", pest_err)),
    }
}


fn parse_expr(pairs: Pairs<Rule>) -> Symbol {
    lazy_static! {
        static ref CLIMBER: PrecClimber<Rule> = PrecClimber::new(vec![
            Operator::new(Rule::lt, Assoc::Left) | Operator::new(Rule::le, Assoc::Left)
              | Operator::new(Rule::gt, Assoc::Left) | Operator::new(Rule::ge, Assoc::Left)
              | Operator::new(Rule::eq, Assoc::Left) | Operator::new(Rule::ne, Assoc::Left),
            Operator::new(Rule::add, Assoc::Left) | Operator::new(Rule::sub, Assoc::Left),
            Operator::new(Rule::mul, Assoc::Left) | Operator::new(Rule::div, Assoc::Left),
        ]);
    }
    CLIMBER.climb(pairs, make_symbol, |lhs, op, rhs| {
        let target = match op.as_rule() {
            Rule::lt => "less-than",
            Rule::le => "less-than-or-equal",
            Rule::gt => "greater-than",
            Rule::ge => "greater-than-or-equal",
            Rule::eq => "equals",
            Rule::ne => "not-equal",
            Rule::add => "add",
            Rule::sub => "sub",
            Rule::mul => "mul",
            Rule::div => "div",
            _ => unreachable!("encountered rule `{:?}` while climbing", op.as_rule()),
        }.to_string().into();
        Symbol::Apply { target: Box::new(target), args: vec![lhs, rhs] }
    })
}

fn parse_as_args(pairs: Pairs<Rule>) -> Vec<Symbol> {
    pairs
        .map(make_symbol)
        .collect()
}
fn make_symbol(pair: Pair<Rule>) -> Symbol {
    match pair.as_rule() {
        Rule::num => pair.as_str().parse::<KeyType>().expect("Rule::num failed to parse").into(),
        Rule::dice => {
            let items = pair
                .as_str()
                .split('d')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<KeyType>().expect("Rule::dice failed to parse"))
                .collect::<Vec<KeyType>>();
            if items.len() == 1 {
                Distr::unif(items[0])
            } else {
                Distr::stacked_unifs(items[0], items[1])
            }.into()
        }
        Rule::decimal_prob => pair.as_str().parse::<ProbType>().expect("Rule::decimal_prob failed to parse").into(),
        Rule::percent_prob =>
            (pair
             .as_str()[0..pair.as_str().len()-1]
             .parse::<ProbType>()
             .expect("Rule::percent_prob failed to parse") / 100.0)
            .into(),
        Rule::ident => pair.as_str().to_string().into(),
        Rule::expr => parse_expr(pair.into_inner()),
        Rule::range_to => Symbol::Apply{
            target: Box::new("range-to".to_string().into()),
            args: parse_as_args(pair.into_inner())
        },
        Rule::repeats => Symbol::Apply{
            target: Box::new("repeat".to_string().into()),
            args: parse_as_args(pair.into_inner())
        },
        Rule::fn_lit => unreachable!("fn literal not supported"),
        Rule::seq => Symbol::Seq(pair.into_inner()
                                     .map(make_symbol)
                                     .collect()),
        Rule::fn_call => {
            let mut pairs = pair.into_inner();
            let target = pairs
                .next()
                .expect("fn_call needs target string")
                .as_str()
                .to_string()
                .into();
            Symbol::Apply { target: Box::new(target), args: parse_as_args(pairs) }
        },
        Rule::assignment => {
            let mut pairs = pair.into_inner();
            Symbol::Assigner {
                name: pairs.next().expect("Rule::assignment missing name").as_str().to_string(),
                def_type: None,
                expr: Box::new(make_symbol(pairs.next().expect("Rule::assignment missing expr"))),
            }
        }
        Rule::assignment_with_type => {
            let mut pairs = pair.into_inner();
            Symbol::Assigner {
                name: pairs.next().expect("Rule::assignment missing name").as_str().to_string(),
                def_type: Some(pairs.next().expect("Rule::assignment_with_type missing type").as_str().to_string()),
                expr: Box::new(make_symbol(pairs.next().expect("Rule::assignment missing expr"))),
            }
        }
             Rule::add | Rule::sub | Rule::mul  | Rule::div
           | Rule::lt | Rule::le | Rule::gt | Rule::ge | Rule::eq | Rule::ne
           | Rule::pre_zero | Rule::pre_two_one | Rule::pre_three
           | Rule::parens | Rule::term | Rule::op | Rule::eoi | Rule::line
           | Rule::WHITESPACE | Rule::COMMENT => unreachable!("reached unreachable rule: {:?}", pair.as_rule()),
    }
}