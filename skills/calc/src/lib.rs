#![forbid(unsafe_code)]

use bigdecimal::BigDecimal;
use bigdecimal::Zero;
use mediatype::{media_type, MediaType};
use pest::iterators::{Pair, Pairs};
use pest::prec_climber::*;
use pest::Parser;
use std::collections::VecDeque;
use thiserror::Error;
use yozuk_sdk::prelude::*;

pub const ENTRY: SkillEntry = SkillEntry {
    model_id: b"Bk4CKgQi8qhO3A0IBqK5t",
    config_schema: None,
    init: |_, _| {
        Skill::builder()
            .add_preprocessor(CalcPreprocessor)
            .add_translator(CalcTranslator)
            .set_command(CalcCommand)
            .build()
    },
};

#[derive(Debug)]
struct CalcPreprocessor;

impl Preprocessor for CalcPreprocessor {
    fn preprocess(&self, input: Vec<Token>) -> Vec<Token> {
        let mut output = Vec::new();
        let mut tokens = input.into_iter().collect::<VecDeque<_>>();
        while !tokens.is_empty() {
            for i in 1..=tokens.len() {
                let len = tokens.len() + 1 - i;
                let exp = tokens
                    .iter()
                    .take(len)
                    .map(|token| token.as_utf8())
                    .collect::<Vec<_>>();
                let exp = exp.join("");
                let is_exp = CalcParser::parse(Rule::calculation, &exp).is_ok()
                    && (len >= 2 || CalcParser::parse(Rule::single_num, &exp).is_err());
                if is_exp {
                    for _ in 0..len {
                        tokens.pop_front();
                    }
                    output.push(tk!(exp, "text/vnd.yozuk.calc"));
                    break;
                }
            }
            if let Some(front) = tokens.pop_front() {
                output.push(front);
            }
        }
        output
    }
}

#[derive(pest_derive::Parser)]
#[grammar = "calc.pest"]
pub struct CalcParser;

lazy_static::lazy_static! {
    static ref PREC_CLIMBER: PrecClimber<Rule> = {
        use Assoc::*;
        use Rule::*;

        PrecClimber::new(vec![
            Operator::new(add, Left) | Operator::new(subtract, Left),
            Operator::new(multiply, Left) | Operator::new(divide, Left),
        ])
    };
}

fn eval(expression: Pairs<Rule>) -> Result<BigDecimal, CalcError> {
    PREC_CLIMBER.climb(
        expression,
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::num => Ok(pair.as_str().parse::<BigDecimal>().unwrap()),
            Rule::expr => eval(pair.into_inner()),
            _ => unreachable!(),
        },
        |lhs: Result<BigDecimal, CalcError>, op: Pair<Rule>, rhs: Result<BigDecimal, CalcError>| {
            let lhs = lhs?;
            let rhs = rhs?;
            Ok(match op.as_rule() {
                Rule::add => lhs + rhs,
                Rule::subtract => lhs - rhs,
                Rule::multiply => lhs * rhs,
                Rule::divide if rhs.is_zero() => return Err(CalcError::DivisionByZero),
                Rule::divide => lhs / rhs,
                _ => unreachable!(),
            })
        },
    )
}

#[derive(Error, Debug, Clone)]
pub enum CalcError {
    #[error("Division by zero")]
    DivisionByZero,
}

#[derive(Debug)]
pub struct CalcTranslator;

impl Translator for CalcTranslator {
    fn parse(&self, args: &[Token]) -> Option<CommandArgs> {
        let media_type = MediaType::parse("text/vnd.yozuk.calc").unwrap();
        if args.iter().any(|arg| arg.media_type != media_type) {
            return None;
        }
        let exp = args
            .iter()
            .filter(|arg| arg.media_type == media_type)
            .map(|arg| arg.as_utf8())
            .collect::<Vec<_>>();
        if exp.len() == 1 {
            Some(CommandArgs::new().add_args([exp[0]]))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct CalcCommand;

impl Command for CalcCommand {
    fn run(&self, args: CommandArgs) -> Result<Output, Output> {
        let rule = CalcParser::parse(Rule::calculation, &args.args[1]).unwrap();
        eval(rule)
            .map(|result| Output {
                module: "Calculator".into(),
                sections: vec![Section::new(
                    format!("{}", result),
                    media_type!(TEXT / PLAIN),
                )],
            })
            .map_err(|err| Output {
                module: "Calculator".into(),
                sections: vec![Section::new(format!("{}", err), media_type!(TEXT / PLAIN))
                    .kind(SectionKind::Comment)],
            })
    }
}
