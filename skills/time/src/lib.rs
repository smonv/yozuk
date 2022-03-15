#![forbid(unsafe_code)]
#![deny(clippy::all)]

use chrono::prelude::*;
use clap::Parser;
use mediatype::media_type;
use yozuk_helper_english::normalized_eq;
use yozuk_sdk::prelude::*;

pub const ENTRY: SkillEntry = SkillEntry {
    model_id: b"1gLomuDRfB5vTIsa6ouuX",
    config_schema: None,
    init: |_, _| {
        Skill::builder()
            .add_corpus(TimeCorpus)
            .add_translator(TimeTranslator)
            .set_command(TimeCommand)
            .build()
    },
};

#[derive(Debug)]
pub struct TimeCorpus;

impl Corpus for TimeCorpus {
    fn training_data(&self) -> Vec<Vec<Token>> {
        vec![
            tk!(["time"; "time:keyword"]),
            tk!(["current", "time"; "time:keyword"]),
            tk!(["What", "time"; "time:keyword", "is", "it", "now"]),
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Debug)]
pub struct TimeTranslator;

impl Translator for TimeTranslator {
    fn parse(&self, args: &[Token]) -> Option<CommandArgs> {
        let time = args
            .iter()
            .any(|arg| arg.tag == "time:keyword" && normalized_eq(arg.as_utf8(), &["time"], 1));

        if time {
            return Some(CommandArgs::new());
        }

        None
    }
}

#[derive(Debug)]
pub struct TimeCommand;

impl Command for TimeCommand {
    fn run(&self, args: CommandArgs) -> Result<Output, Output> {
        let _args = Args::try_parse_from(args.args).unwrap();
        Ok(Output {
            module: "Time".into(),
            sections: vec![Section::new(
                Local::now().to_rfc3339(),
                media_type!(TEXT / PLAIN),
            )],
        })
    }
}

#[derive(Parser)]
pub struct Args {}
